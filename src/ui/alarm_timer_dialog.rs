//! Alarm and Timer list management dialog
//!
//! This dialog provides full control over multiple alarms and timers using the global manager.
//! Timers can be edited inline when stopped. Changes are reflected on all clock displays.

use crate::core::{global_timer_manager, play_preview_sound, stop_all_sounds, AlarmConfig, TimerConfig, TimerState};
use gtk4::prelude::*;
use gtk4::{
    Adjustment, Box as GtkBox, Button, CheckButton, Entry, FileDialog, FileFilter, Frame, Label,
    ListBox, ListBoxRow, Orientation, ScrolledWindow, Separator, SpinButton, Window,
};
use gtk4::glib::WeakRef;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static ALARM_TIMER_DIALOG: RefCell<Option<WeakRef<Window>>> = const { RefCell::new(None) };
}

/// Close the alarm/timer dialog if it's open
pub fn close_alarm_timer_dialog() {
    ALARM_TIMER_DIALOG.with(|dialog_ref| {
        let mut dialog_opt = dialog_ref.borrow_mut();
        if let Some(weak) = dialog_opt.take() {
            if let Some(dialog) = weak.upgrade() {
                dialog.close();
            }
        }
    });
}

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

/// Alarm and Timer list management dialog using global manager
pub struct AlarmTimerDialog {
    window: Window,
    alarm_list_box: ListBox,
    timer_list_box: ListBox,
    refresh_source_id: Rc<RefCell<Option<gtk4::glib::SourceId>>>,
}

impl AlarmTimerDialog {
    pub fn new(parent: Option<&Window>) -> Self {
        let window = Window::builder()
            .title("Alarms & Timers")
            .modal(false)
            .default_width(500)
            .default_height(600)
            .resizable(true)
            .build();

        if let Some(parent) = parent {
            window.set_transient_for(Some(parent));
        }

        let main_box = GtkBox::new(Orientation::Vertical, 12);
        main_box.set_margin_start(12);
        main_box.set_margin_end(12);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);

        // ===== TIMER SECTION =====
        let timer_frame = Frame::new(Some("Timers"));
        let timer_vbox = GtkBox::new(Orientation::Vertical, 8);
        timer_vbox.set_margin_start(8);
        timer_vbox.set_margin_end(8);
        timer_vbox.set_margin_top(8);
        timer_vbox.set_margin_bottom(8);

        let timer_scroll = ScrolledWindow::new();
        timer_scroll.set_min_content_height(200);
        timer_scroll.set_vexpand(true);

        let timer_list_box = ListBox::new();
        timer_list_box.set_selection_mode(gtk4::SelectionMode::None);
        timer_list_box.add_css_class("boxed-list");
        timer_scroll.set_child(Some(&timer_list_box));
        timer_vbox.append(&timer_scroll);

        // Add timer button row with global sound setting
        let add_row = GtkBox::new(Orientation::Horizontal, 8);

        let add_timer_button = Button::with_label("+ Add Timer");
        add_row.append(&add_timer_button);

        // Global timer sound setting
        add_row.append(&Separator::new(Orientation::Vertical));
        let sound_check = CheckButton::with_label("Sound");
        let sound_path_label = Label::new(None::<&str>);
        sound_path_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        sound_path_label.set_hexpand(true);
        sound_path_label.set_halign(gtk4::Align::Start);

        // Initialize from global manager
        if let Ok(manager) = global_timer_manager().read() {
            let sound = manager.get_global_timer_sound();
            sound_check.set_active(sound.enabled);
            sound_path_label.set_text(sound.custom_sound_path.as_deref().unwrap_or("System default"));
        }

        add_row.append(&sound_check);
        add_row.append(&sound_path_label);

        let browse_sound_btn = Button::with_label("...");
        browse_sound_btn.set_tooltip_text(Some("Browse for sound file"));
        add_row.append(&browse_sound_btn);

        let preview_timer_sound_btn = Button::new();
        preview_timer_sound_btn.set_icon_name("audio-speakers-symbolic");
        preview_timer_sound_btn.set_tooltip_text(Some("Preview timer sound"));
        add_row.append(&preview_timer_sound_btn);

        let stop_timer_sound_btn = Button::new();
        stop_timer_sound_btn.set_icon_name("media-playback-stop-symbolic");
        stop_timer_sound_btn.set_tooltip_text(Some("Stop sound"));
        add_row.append(&stop_timer_sound_btn);

        timer_vbox.append(&add_row);

        timer_frame.set_child(Some(&timer_vbox));
        main_box.append(&timer_frame);

        // ===== ALARM SECTION =====
        let alarm_frame = Frame::new(Some("Alarms"));
        let alarm_vbox = GtkBox::new(Orientation::Vertical, 8);
        alarm_vbox.set_margin_start(8);
        alarm_vbox.set_margin_end(8);
        alarm_vbox.set_margin_top(8);
        alarm_vbox.set_margin_bottom(8);

        let alarm_scroll = ScrolledWindow::new();
        alarm_scroll.set_min_content_height(150);
        alarm_scroll.set_vexpand(true);

        let alarm_list_box = ListBox::new();
        alarm_list_box.set_selection_mode(gtk4::SelectionMode::None);
        alarm_list_box.add_css_class("boxed-list");
        alarm_scroll.set_child(Some(&alarm_list_box));
        alarm_vbox.append(&alarm_scroll);

        let add_alarm_button = Button::with_label("+ Add Alarm");
        add_alarm_button.set_halign(gtk4::Align::Start);
        alarm_vbox.append(&add_alarm_button);

        alarm_frame.set_child(Some(&alarm_vbox));
        main_box.append(&alarm_frame);

        // ===== CLOSE BUTTON =====
        main_box.append(&Separator::new(Orientation::Horizontal));

        let close_button = Button::with_label("Close");
        close_button.set_halign(gtk4::Align::End);
        close_button.set_margin_top(8);
        main_box.append(&close_button);

        window.set_child(Some(&main_box));

        let dialog = Self {
            window: window.clone(),
            alarm_list_box: alarm_list_box.clone(),
            timer_list_box: timer_list_box.clone(),
            refresh_source_id: Rc::new(RefCell::new(None)),
        };

        // Populate lists
        dialog.refresh_timer_list();
        dialog.refresh_alarm_list();

        // Set up periodic refresh only for running timer countdown display (1 second)
        // This is less aggressive and only needed to update running timer displays
        // Use weak references so the timer stops when widgets are destroyed
        let timer_list_box_weak = timer_list_box.downgrade();
        let alarm_list_box_weak = alarm_list_box.downgrade();
        let refresh_id = gtk4::glib::timeout_add_local(
            std::time::Duration::from_secs(1),
            move || {
                // Check if widgets are still alive - stop timer if not
                let Some(timer_list_box) = timer_list_box_weak.upgrade() else {
                    return gtk4::glib::ControlFlow::Break;
                };
                let Some(alarm_list_box) = alarm_list_box_weak.upgrade() else {
                    return gtk4::glib::ControlFlow::Break;
                };

                // Skip if widgets are not visible (window minimized or hidden)
                if !timer_list_box.is_mapped() {
                    return gtk4::glib::ControlFlow::Continue;
                }

                // Use try_read to avoid blocking GTK main thread
                // If lock is held, skip this refresh cycle
                let needs_refresh = if let Ok(manager) = global_timer_manager().try_read() {
                    manager.timers.iter().any(|t| {
                        t.state == TimerState::Running || t.state == TimerState::Paused || t.state == TimerState::Finished
                    }) || !manager.triggered_alarms.is_empty()
                } else {
                    false // Lock is busy, skip this cycle
                };

                if needs_refresh {
                    Self::refresh_timer_list_static(&timer_list_box);
                    Self::refresh_alarm_list_static(&alarm_list_box);
                }
                gtk4::glib::ControlFlow::Continue
            },
        );
        *dialog.refresh_source_id.borrow_mut() = Some(refresh_id);

        // Global sound checkbox handler
        sound_check.connect_toggled(move |check| {
            if let Ok(mut manager) = global_timer_manager().write() {
                let mut sound = manager.get_global_timer_sound().clone();
                sound.enabled = check.is_active();
                manager.set_global_timer_sound(sound);
            }
        });

        // Browse sound file handler
        let sound_path_label_for_browse = sound_path_label.clone();
        browse_sound_btn.connect_clicked(move |btn| {
            let filter = FileFilter::new();
            filter.add_mime_type("audio/*");
            filter.set_name(Some("Audio files"));

            let filters = gtk4::gio::ListStore::new::<FileFilter>();
            filters.append(&filter);

            let file_dialog = FileDialog::builder()
                .title("Select Timer Sound File")
                .filters(&filters)
                .build();

            let lbl = sound_path_label_for_browse.clone();
            let win = btn.root().and_downcast::<Window>();

            file_dialog.open(win.as_ref(), gtk4::gio::Cancellable::NONE, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        let path_str = path.to_string_lossy().to_string();
                        lbl.set_text(&path_str);
                        if let Ok(mut manager) = global_timer_manager().write() {
                            let mut sound = manager.get_global_timer_sound().clone();
                            sound.custom_sound_path = Some(path_str);
                            manager.set_global_timer_sound(sound);
                        }
                    }
                }
            });
        });

        // Preview timer sound handler
        preview_timer_sound_btn.connect_clicked(move |_| {
            // First stop any currently playing sound
            stop_all_sounds();
            // Then play the preview
            if let Ok(manager) = global_timer_manager().read() {
                let sound = manager.get_global_timer_sound().clone();
                play_preview_sound(&sound);
            }
        });

        // Stop timer sound handler
        stop_timer_sound_btn.connect_clicked(move |_| {
            stop_all_sounds();
        });

        // Add timer button
        let timer_list_for_add = timer_list_box.clone();
        add_timer_button.connect_clicked(move |_| {
            let new_timer = TimerConfig::new();
            if let Ok(mut manager) = global_timer_manager().write() {
                manager.add_timer(new_timer);
            }
            Self::refresh_timer_list_static(&timer_list_for_add);
        });

        // Add alarm button
        let alarm_list_for_add = alarm_list_box.clone();
        add_alarm_button.connect_clicked(move |_| {
            let new_alarm = AlarmConfig::new();
            if let Ok(mut manager) = global_timer_manager().write() {
                manager.add_alarm(new_alarm);
            }
            Self::refresh_alarm_list_static(&alarm_list_for_add);
        });

        // Close button
        let window_clone = window.clone();
        let refresh_id_for_close = dialog.refresh_source_id.clone();
        close_button.connect_clicked(move |_| {
            if let Some(id) = refresh_id_for_close.borrow_mut().take() {
                id.remove();
            }
            window_clone.close();
        });

        // Clear singleton reference on close-request (before destroy)
        // This prevents the "window shown after destroyed" warning
        window.connect_close_request(move |_| {
            ALARM_TIMER_DIALOG.with(|dialog_ref| {
                *dialog_ref.borrow_mut() = None;
            });
            gtk4::glib::Propagation::Proceed
        });

        // Stop refresh timer when window is destroyed
        let refresh_id_for_destroy = dialog.refresh_source_id.clone();
        window.connect_destroy(move |_| {
            if let Some(id) = refresh_id_for_destroy.borrow_mut().take() {
                id.remove();
            }
        });

        dialog
    }

    /// Show the Alarm/Timer dialog (singleton pattern - only one instance at a time)
    pub fn show(parent: Option<&Window>) {
        ALARM_TIMER_DIALOG.with(|dialog_ref| {
            let mut dialog_opt = dialog_ref.borrow_mut();

            // Check if dialog already exists and is still alive and visible
            if let Some(weak) = dialog_opt.as_ref() {
                if let Some(window) = weak.upgrade() {
                    // Only reuse if window is still mapped (not being destroyed)
                    if window.is_mapped() {
                        window.present();  // Bring to front
                        return;
                    }
                }
            }

            // Clear any stale reference
            *dialog_opt = None;

            // Create new dialog
            let dialog = Self::new(parent);
            *dialog_opt = Some(dialog.window.downgrade());
            dialog.window.present();
        });
    }

    fn refresh_timer_list(&self) {
        Self::refresh_timer_list_static(&self.timer_list_box);
    }

    fn refresh_timer_list_static(list_box: &ListBox) {
        // Clear existing rows
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        let timers = if let Ok(manager) = global_timer_manager().read() {
            manager.timers.clone()
        } else {
            Vec::new()
        };

        if timers.is_empty() {
            let empty_label = Label::new(Some("No timers. Click '+ Add Timer' to create one."));
            empty_label.add_css_class("dim-label");
            empty_label.set_margin_top(12);
            empty_label.set_margin_bottom(12);
            let row = ListBoxRow::new();
            row.set_child(Some(&empty_label));
            row.set_selectable(false);
            row.set_activatable(false);
            list_box.append(&row);
            return;
        }

        for timer in &timers {
            let row = Self::create_timer_row(timer, list_box);
            list_box.append(&row);
        }
    }

    fn create_timer_row(timer: &TimerConfig, list_box: &ListBox) -> ListBoxRow {
        let row = ListBoxRow::new();
        row.set_selectable(false);
        row.set_activatable(false);

        let timer_id = timer.id.clone();
        let is_stopped = timer.state == TimerState::Stopped;

        let hbox = GtkBox::new(Orientation::Horizontal, 6);
        hbox.set_margin_start(8);
        hbox.set_margin_end(8);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);

        // Play/Pause/Resume button
        let play_pause_btn = Button::new();
        play_pause_btn.set_tooltip_text(Some(match timer.state {
            TimerState::Stopped | TimerState::Finished => "Start",
            TimerState::Running => "Pause",
            TimerState::Paused => "Resume",
        }));
        play_pause_btn.set_icon_name(match timer.state {
            TimerState::Stopped | TimerState::Finished => "media-playback-start-symbolic",
            TimerState::Running => "media-playback-pause-symbolic",
            TimerState::Paused => "media-playback-start-symbolic",
        });
        if timer.state == TimerState::Stopped || timer.state == TimerState::Finished {
            play_pause_btn.add_css_class("suggested-action");
        }
        let tid = timer_id.clone();
        let lb = list_box.clone();
        play_pause_btn.connect_clicked(move |_| {
            // Read current state from manager at click time
            let action_taken = if let Ok(mut manager) = global_timer_manager().write() {
                let current_state = manager.timers.iter()
                    .find(|t| t.id == tid)
                    .map(|t| t.state)
                    .unwrap_or(TimerState::Stopped);
                match current_state {
                    TimerState::Stopped | TimerState::Finished => manager.start_timer(&tid),
                    TimerState::Running => manager.pause_timer(&tid),
                    TimerState::Paused => manager.resume_timer(&tid),
                }
                true
            } else {
                false
            };
            if action_taken {
                Self::refresh_timer_list_static(&lb);
            }
        });
        hbox.append(&play_pause_btn);

        // Stop button
        let stop_btn = Button::new();
        stop_btn.set_icon_name("media-playback-stop-symbolic");
        stop_btn.set_tooltip_text(Some("Stop/Reset"));
        stop_btn.set_sensitive(timer.state != TimerState::Stopped);
        let tid = timer_id.clone();
        let lb = list_box.clone();
        stop_btn.connect_clicked(move |_| {
            let action_taken = if let Ok(mut manager) = global_timer_manager().write() {
                manager.stop_timer(&tid);
                true
            } else {
                false
            };
            if action_taken {
                Self::refresh_timer_list_static(&lb);
            }
        });
        hbox.append(&stop_btn);

        // H:M:S spinners (editable when stopped)
        let hours = (timer.countdown_duration / 3600) as f64;
        let mins = ((timer.countdown_duration % 3600) / 60) as f64;
        let secs = (timer.countdown_duration % 60) as f64;

        let hour_adj = Adjustment::new(hours, 0.0, 99.0, 1.0, 1.0, 0.0);
        let hour_spin = SpinButton::new(Some(&hour_adj), 1.0, 0);
        hour_spin.set_width_chars(2);
        hour_spin.set_sensitive(is_stopped);
        hbox.append(&Label::new(Some("H :")));
        hbox.append(&hour_spin);

        let min_adj = Adjustment::new(mins, 0.0, 59.0, 1.0, 5.0, 0.0);
        let min_spin = SpinButton::new(Some(&min_adj), 1.0, 0);
        min_spin.set_width_chars(2);
        min_spin.set_sensitive(is_stopped);
        hbox.append(&Label::new(Some("M :")));
        hbox.append(&min_spin);

        let sec_adj = Adjustment::new(secs, 0.0, 59.0, 1.0, 5.0, 0.0);
        let sec_spin = SpinButton::new(Some(&sec_adj), 1.0, 0);
        sec_spin.set_width_chars(2);
        sec_spin.set_sensitive(is_stopped);
        hbox.append(&Label::new(Some("S :")));
        hbox.append(&sec_spin);

        // Current time display (when running/paused/finished)
        if !is_stopped {
            let time_str = timer.display_string();
            let time_label = Label::new(Some(&time_str));
            time_label.add_css_class("monospace");
            time_label.set_width_chars(10);
            match timer.state {
                TimerState::Running => time_label.add_css_class("success"),
                TimerState::Paused => time_label.add_css_class("warning"),
                TimerState::Finished => time_label.add_css_class("error"),
                _ => {}
            }
            hbox.append(&time_label);
        }

        // Spacer
        let spacer = GtkBox::new(Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        hbox.append(&spacer);

        // Delete button
        let delete_btn = Button::new();
        delete_btn.set_icon_name("user-trash-symbolic");
        delete_btn.set_tooltip_text(Some("Delete timer"));
        delete_btn.add_css_class("destructive-action");
        let tid = timer_id.clone();
        let lb = list_box.clone();
        delete_btn.connect_clicked(move |_| {
            let deleted = if let Ok(mut manager) = global_timer_manager().write() {
                manager.remove_timer(&tid);
                true
            } else {
                false
            };
            if deleted {
                Self::refresh_timer_list_static(&lb);
            }
        });
        hbox.append(&delete_btn);

        // Connect spinner value changes (only apply when stopped)
        if is_stopped {
            let tid = timer_id.clone();
            let hour_spin_c = hour_spin.clone();
            let min_spin_c = min_spin.clone();
            let sec_spin_c = sec_spin.clone();

            let apply_duration = Rc::new(move || {
                let duration = (hour_spin_c.value() as u64) * 3600
                    + (min_spin_c.value() as u64) * 60
                    + sec_spin_c.value() as u64;
                if let Ok(mut manager) = global_timer_manager().write() {
                    manager.update_timer(&tid, |t| {
                        t.countdown_duration = duration;
                    });
                }
            });

            let apply = apply_duration.clone();
            hour_spin.connect_value_changed(move |_| apply());

            let apply = apply_duration.clone();
            min_spin.connect_value_changed(move |_| apply());

            let apply = apply_duration.clone();
            sec_spin.connect_value_changed(move |_| apply());
        }

        row.set_child(Some(&hbox));
        row
    }

    fn refresh_alarm_list(&self) {
        Self::refresh_alarm_list_static(&self.alarm_list_box);
    }

    fn refresh_alarm_list_static(list_box: &ListBox) {
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        let (alarms, triggered_ids) = if let Ok(manager) = global_timer_manager().read() {
            (manager.alarms.clone(), manager.triggered_alarms.clone())
        } else {
            (Vec::new(), std::collections::HashSet::new())
        };

        if alarms.is_empty() {
            let empty_label = Label::new(Some("No alarms. Click '+ Add Alarm' to create one."));
            empty_label.add_css_class("dim-label");
            empty_label.set_margin_top(12);
            empty_label.set_margin_bottom(12);
            let row = ListBoxRow::new();
            row.set_child(Some(&empty_label));
            row.set_selectable(false);
            row.set_activatable(false);
            list_box.append(&row);
            return;
        }

        for alarm in &alarms {
            let is_triggered = triggered_ids.contains(&alarm.id);
            let row = Self::create_alarm_row(alarm, is_triggered, list_box.clone());
            list_box.append(&row);
        }
    }

    fn create_alarm_row(alarm: &AlarmConfig, is_triggered: bool, list_box: ListBox) -> ListBoxRow {
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
        let aid_for_toggle = alarm_id.clone();
        enabled_check.connect_toggled(move |check| {
            if let Ok(mut manager) = global_timer_manager().write() {
                manager.update_alarm(&aid_for_toggle, |a| {
                    a.enabled = check.is_active();
                });
            }
        });
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

        // Triggered indicator
        if is_triggered {
            let triggered_label = Label::new(Some("RINGING"));
            triggered_label.add_css_class("error");
            hbox.append(&triggered_label);

            let dismiss_btn = Button::with_label("Dismiss");
            dismiss_btn.add_css_class("destructive-action");
            let aid = alarm_id.clone();
            let lb = list_box.clone();
            dismiss_btn.connect_clicked(move |_| {
                let dismissed = if let Ok(mut manager) = global_timer_manager().write() {
                    manager.dismiss_alarm(&aid);
                    true
                } else {
                    false
                };
                if dismissed {
                    Self::refresh_alarm_list_static(&lb);
                }
            });
            hbox.append(&dismiss_btn);
        }

        // Edit button
        let edit_btn = Button::new();
        edit_btn.set_icon_name("document-edit-symbolic");
        edit_btn.set_tooltip_text(Some("Edit alarm"));
        let alarm_clone = alarm.clone();
        let lb_edit = list_box.clone();
        edit_btn.connect_clicked(move |btn| {
            Self::show_alarm_edit_dialog(&alarm_clone, lb_edit.clone(), btn.root().and_downcast::<Window>().as_ref());
        });
        hbox.append(&edit_btn);

        // Delete button
        let delete_btn = Button::new();
        delete_btn.set_icon_name("user-trash-symbolic");
        delete_btn.set_tooltip_text(Some("Delete alarm"));
        delete_btn.add_css_class("destructive-action");
        let aid_del = alarm_id.clone();
        let lb = list_box.clone();
        delete_btn.connect_clicked(move |_| {
            let deleted = if let Ok(mut manager) = global_timer_manager().write() {
                manager.remove_alarm(&aid_del);
                true
            } else {
                false
            };
            if deleted {
                Self::refresh_alarm_list_static(&lb);
            }
        });
        hbox.append(&delete_btn);

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
        sorted_days
            .iter()
            .map(|&d| *day_names.get(d as usize).unwrap_or(&"?"))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn show_alarm_edit_dialog(alarm: &AlarmConfig, list_box: ListBox, parent: Option<&Window>) {
        let dialog = Window::builder()
            .title("Edit Alarm")
            .modal(true)
            .default_width(400)
            .default_height(450)
            .build();

        if let Some(parent) = parent {
            dialog.set_transient_for(Some(parent));
        }

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

        // Sound file
        let sound_box = GtkBox::new(Orientation::Horizontal, 6);
        sound_box.append(&Label::new(Some("Sound:")));
        let sound_label = Label::new(Some(
            alarm.sound.custom_sound_path.as_deref().unwrap_or("System default")
        ));
        sound_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        sound_label.set_hexpand(true);
        sound_box.append(&sound_label);

        let browse_btn = Button::with_label("Browse...");
        let sound_label_for_browse = sound_label.clone();
        let alarm_id_for_sound = alarm_id.clone();
        browse_btn.connect_clicked(move |btn| {
            let filter = FileFilter::new();
            filter.add_mime_type("audio/*");
            filter.set_name(Some("Audio files"));

            let filters = gtk4::gio::ListStore::new::<FileFilter>();
            filters.append(&filter);

            let file_dialog = FileDialog::builder()
                .title("Select Sound File")
                .filters(&filters)
                .build();

            let lbl = sound_label_for_browse.clone();
            let aid = alarm_id_for_sound.clone();
            let win = btn.root().and_downcast::<Window>();

            file_dialog.open(win.as_ref(), gtk4::gio::Cancellable::NONE, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        let path_str = path.to_string_lossy().to_string();
                        lbl.set_text(&path_str);
                        if let Ok(mut manager) = global_timer_manager().write() {
                            manager.update_alarm(&aid, |a| {
                                a.sound.custom_sound_path = Some(path_str);
                            });
                        }
                    }
                }
            });
        });
        sound_box.append(&browse_btn);

        // Preview alarm sound button
        let preview_alarm_btn = Button::new();
        preview_alarm_btn.set_icon_name("audio-speakers-symbolic");
        preview_alarm_btn.set_tooltip_text(Some("Preview alarm sound"));
        let alarm_id_for_preview = alarm.id.clone();
        preview_alarm_btn.connect_clicked(move |_| {
            // Stop any currently playing sound
            stop_all_sounds();
            // Play this alarm's sound
            if let Ok(manager) = global_timer_manager().read() {
                if let Some(alarm) = manager.alarms.iter().find(|a| a.id == alarm_id_for_preview) {
                    play_preview_sound(&alarm.sound);
                }
            }
        });
        sound_box.append(&preview_alarm_btn);

        let stop_alarm_btn = Button::new();
        stop_alarm_btn.set_icon_name("media-playback-stop-symbolic");
        stop_alarm_btn.set_tooltip_text(Some("Stop sound"));
        stop_alarm_btn.connect_clicked(move |_| {
            stop_all_sounds();
        });
        sound_box.append(&stop_alarm_btn);

        vbox.append(&sound_box);

        // Visual flash
        let flash_check = CheckButton::with_label("Visual flash");
        flash_check.set_active(alarm.sound.visual_enabled);
        vbox.append(&flash_check);

        // Buttons
        vbox.append(&Separator::new(Orientation::Horizontal));
        let button_box = GtkBox::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::End);

        let cancel_btn = Button::with_label("Cancel");
        let dialog_for_cancel = dialog.clone();
        cancel_btn.connect_clicked(move |_| dialog_for_cancel.close());
        button_box.append(&cancel_btn);

        let save_btn = Button::with_label("Save");
        save_btn.add_css_class("suggested-action");
        let dialog_for_save = dialog.clone();
        save_btn.connect_clicked(move |_| {
            if let Ok(mut manager) = global_timer_manager().write() {
                manager.update_alarm(&alarm_id, |a| {
                    a.hour = hour_spin.value() as u32;
                    a.minute = min_spin.value() as u32;
                    a.second = sec_spin.value() as u32;

                    let mut days = Vec::new();
                    for (i, check) in day_checks.iter().enumerate() {
                        if check.is_active() {
                            days.push(i as u32);
                        }
                    }
                    a.days = days;

                    let label_text = label_entry.text().to_string();
                    a.label = if label_text.is_empty() { None } else { Some(label_text) };
                    a.sound.enabled = sound_check.is_active();
                    a.sound.visual_enabled = flash_check.is_active();
                });
            }
            Self::refresh_alarm_list_static(&list_box);
            dialog_for_save.close();
        });
        button_box.append(&save_btn);
        vbox.append(&button_box);

        dialog.set_child(Some(&vbox));
        dialog.present();
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
}
