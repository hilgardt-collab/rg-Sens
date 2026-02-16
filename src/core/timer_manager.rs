//! Global timer and alarm manager
//!
//! Provides a single shared list of timers and alarms used by all clock display instances.
//! This ensures that timer/alarm state is consistent across the application.

use crate::audio::AudioPlayer;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;
use uuid::Uuid;

/// Trigger cache maintenance to prevent memory leaks
/// Uses glib::idle_add_once which is thread-safe and schedules on the GTK main thread
fn trigger_cache_maintenance() {
    // Schedule cache clearing on the GTK main context since caches are thread-local
    // Use idle_add_once (not idle_add_local_once) so this can be called from any thread
    gtk4::glib::idle_add_once(|| {
        // Clear render caches
        crate::ui::render_cache::clear_all_render_caches();

        // Clear Pango caches
        crate::ui::pango_text::clear_pango_caches();

        // Log cache stats for diagnosis
        let stats = crate::ui::render_cache::get_cache_stats();
        log::info!("Render cache stats after clearing: {}", stats);
    });
}

/// Global timer manager instance
static TIMER_MANAGER: Lazy<Arc<RwLock<TimerAlarmManager>>> =
    Lazy::new(|| Arc::new(RwLock::new(TimerAlarmManager::new())));

/// Audio commands for the audio thread
#[allow(dead_code)]
enum AudioCommand {
    Play(AlarmSoundConfig),
    Stop,
    Shutdown,
}

/// Audio thread state including sender and thread handle for cleanup
#[allow(dead_code)]
struct AudioThreadState {
    sender: Option<Sender<AudioCommand>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

/// Global audio thread state
static AUDIO_THREAD: Lazy<Mutex<AudioThreadState>> = Lazy::new(|| {
    // Spawn audio thread
    let (tx, rx) = channel::<AudioCommand>();

    let handle = std::thread::spawn(move || {
        // Create AudioPlayer in this thread (it's not Send)
        let player = match AudioPlayer::new() {
            Ok(p) => p,
            Err(e) => {
                log::warn!("Failed to create audio player: {:?}", e);
                return;
            }
        };

        loop {
            match rx.recv() {
                Ok(AudioCommand::Play(config)) => {
                    if config.enabled {
                        player.set_volume(config.volume);
                        let result = if let Some(ref path) = config.custom_sound_path {
                            player.play(path)
                        } else {
                            player.play_system_alert()
                        };
                        if let Err(e) = result {
                            log::warn!("Failed to play sound: {:?}", e);
                        }
                    }
                }
                Ok(AudioCommand::Stop) => {
                    player.stop();
                }
                Ok(AudioCommand::Shutdown) => {
                    log::debug!("Audio thread received shutdown signal");
                    break;
                }
                Err(_) => {
                    // Channel closed, exit thread
                    break;
                }
            }
        }
        log::debug!("Audio thread exiting");
    });

    Mutex::new(AudioThreadState {
        sender: Some(tx),
        handle: Some(handle),
    })
});

/// Get the global timer manager
pub fn global_timer_manager() -> Arc<RwLock<TimerAlarmManager>> {
    TIMER_MANAGER.clone()
}

/// Stop all currently playing alarm/timer sounds
pub fn stop_all_sounds() {
    if let Ok(guard) = AUDIO_THREAD.lock() {
        if let Some(ref sender) = guard.sender {
            let _ = sender.send(AudioCommand::Stop);
        }
    }
}

/// Play a preview sound using the given sound config
pub fn play_preview_sound(sound_config: &AlarmSoundConfig) {
    if let Ok(guard) = AUDIO_THREAD.lock() {
        if let Some(ref sender) = guard.sender {
            let _ = sender.send(AudioCommand::Play(sound_config.clone()));
        }
    }
}

/// Shutdown the audio thread gracefully
/// Call this during application exit for clean shutdown
pub fn shutdown_audio_thread() {
    if let Ok(mut guard) = AUDIO_THREAD.lock() {
        // Send shutdown command
        if let Some(ref sender) = guard.sender {
            let _ = sender.send(AudioCommand::Shutdown);
        }
        // Drop sender to close channel
        guard.sender = None;
        // Wait for thread to finish
        if let Some(handle) = guard.handle.take() {
            if let Err(e) = handle.join() {
                log::warn!("Audio thread panicked: {:?}", e);
            }
        }
    }
}

// Re-export timer/alarm types from the types crate
pub use rg_sens_types::timer::{
    AlarmConfig, AlarmSoundConfig, TimerConfig, TimerDisplayConfig, TimerMode, TimerState,
};


/// Global timer and alarm manager
pub struct TimerAlarmManager {
    /// All timers
    pub timers: Vec<TimerConfig>,
    /// All alarms
    pub alarms: Vec<AlarmConfig>,
    /// Global timer sound configuration (used for all timers)
    pub global_timer_sound: AlarmSoundConfig,
    /// IDs of currently triggered alarms
    pub triggered_alarms: HashSet<String>,
    /// Alarms that have played their sound this trigger cycle
    alarm_sound_played: HashSet<String>,
    /// Last check time for each alarm
    last_alarm_check: HashMap<String, (u32, u32, u32)>,
    /// Timers that have played their sound
    timer_sound_played: HashSet<String>,
    /// Next alarm info
    pub next_alarm_time: Option<String>,
    pub next_alarm_id: Option<String>,
    /// Callback for when timer/alarm state changes (for UI updates)
    /// Using HashMap with UUID keys to allow callback removal and prevent memory leaks
    change_callbacks: HashMap<String, Box<dyn Fn() + Send + Sync>>,
}

impl TimerAlarmManager {
    pub fn new() -> Self {
        Self {
            timers: Vec::new(),
            alarms: Vec::new(),
            global_timer_sound: AlarmSoundConfig::default(),
            triggered_alarms: HashSet::new(),
            alarm_sound_played: HashSet::new(),
            last_alarm_check: HashMap::new(),
            timer_sound_played: HashSet::new(),
            next_alarm_time: None,
            next_alarm_id: None,
            change_callbacks: HashMap::new(),
        }
    }

    /// Set global timer sound configuration
    pub fn set_global_timer_sound(&mut self, sound: AlarmSoundConfig) {
        self.global_timer_sound = sound;
        self.notify_change();
    }

    /// Get global timer sound configuration
    pub fn get_global_timer_sound(&self) -> &AlarmSoundConfig {
        &self.global_timer_sound
    }

    /// Register a callback for state changes
    /// Returns a callback ID that can be used to remove the callback later
    ///
    /// # Important
    /// Callers MUST call `remove_callback` when the callback is no longer needed
    /// (e.g., when a displayer is destroyed) to prevent memory leaks.
    pub fn on_change<F>(&mut self, callback: F) -> String
    where
        F: Fn() + Send + Sync + 'static,
    {
        let id = Uuid::new_v4().to_string();
        self.change_callbacks.insert(id.clone(), Box::new(callback));
        id
    }

    /// Remove a previously registered callback by its ID
    /// Returns true if a callback was removed, false if the ID was not found
    pub fn remove_callback(&mut self, callback_id: &str) -> bool {
        self.change_callbacks.remove(callback_id).is_some()
    }

    /// Clear all registered callbacks
    /// Use this during application shutdown to ensure cleanup
    pub fn clear_all_callbacks(&mut self) {
        self.change_callbacks.clear();
    }

    /// Get the number of registered callbacks (for debugging memory leaks)
    pub fn callback_count(&self) -> usize {
        self.change_callbacks.len()
    }

    /// Notify all callbacks of a change
    fn notify_change(&self) {
        for callback in self.change_callbacks.values() {
            callback();
        }
    }

    /// Load timers and alarms from config
    pub fn load_config(&mut self, timers: Vec<TimerConfig>, alarms: Vec<AlarmConfig>) {
        self.load_config_with_sound(timers, alarms, None);
    }

    /// Load timers, alarms, and global timer sound from config
    pub fn load_config_with_sound(
        &mut self,
        timers: Vec<TimerConfig>,
        alarms: Vec<AlarmConfig>,
        global_sound: Option<AlarmSoundConfig>,
    ) {
        // Preserve runtime state for existing timers
        let old_timer_states: HashMap<String, (TimerState, u64, Option<Instant>)> = self
            .timers
            .iter()
            .map(|t| (t.id.clone(), (t.state, t.elapsed_ms, t.start_instant)))
            .collect();

        self.timers = timers;
        self.alarms = alarms;
        if let Some(sound) = global_sound {
            self.global_timer_sound = sound;
        }

        // Restore timer runtime state and ensure mode is Countdown
        for timer in &mut self.timers {
            // Force all timers to countdown mode
            timer.mode = TimerMode::Countdown;
            if let Some((state, elapsed, instant)) = old_timer_states.get(&timer.id) {
                timer.state = *state;
                timer.elapsed_ms = *elapsed;
                timer.start_instant = *instant;
            }
        }

        self.notify_change();
    }

    /// Add a new timer
    pub fn add_timer(&mut self, timer: TimerConfig) {
        self.timers.push(timer);
        self.notify_change();
    }

    /// Remove a timer by ID
    pub fn remove_timer(&mut self, timer_id: &str) {
        self.stop_timer(timer_id);
        self.timers.retain(|t| t.id != timer_id);
        self.notify_change();
    }

    /// Update a timer's configuration (when paused/stopped)
    pub fn update_timer(&mut self, timer_id: &str, update_fn: impl FnOnce(&mut TimerConfig)) {
        if let Some(timer) = self.timers.iter_mut().find(|t| t.id == timer_id) {
            // Only allow editing when not running
            if timer.state != TimerState::Running {
                update_fn(timer);
                // Reset elapsed time if stopped
                if timer.state == TimerState::Stopped {
                    timer.elapsed_ms = timer.countdown_duration * 1000;
                }
                self.notify_change();
            }
        }
    }

    /// Start a timer by ID
    pub fn start_timer(&mut self, timer_id: &str) {
        if let Some(timer) = self.timers.iter_mut().find(|t| t.id == timer_id) {
            if timer.state == TimerState::Stopped || timer.state == TimerState::Finished {
                // Initialize elapsed time for countdown
                timer.elapsed_ms = timer.countdown_duration * 1000;
                self.timer_sound_played.remove(timer_id);
            }
            timer.state = TimerState::Running;
            timer.start_instant = Some(Instant::now());
            self.notify_change();
        }
    }

    /// Pause a timer by ID
    pub fn pause_timer(&mut self, timer_id: &str) {
        if let Some(timer) = self.timers.iter_mut().find(|t| t.id == timer_id) {
            timer.state = TimerState::Paused;
            timer.start_instant = None;
            self.notify_change();
        }
    }

    /// Resume a timer by ID
    pub fn resume_timer(&mut self, timer_id: &str) {
        if let Some(timer) = self.timers.iter_mut().find(|t| t.id == timer_id) {
            timer.state = TimerState::Running;
            timer.start_instant = Some(Instant::now());
            self.notify_change();
        }
    }

    /// Stop and reset a timer by ID
    pub fn stop_timer(&mut self, timer_id: &str) {
        if let Some(timer) = self.timers.iter_mut().find(|t| t.id == timer_id) {
            timer.state = TimerState::Stopped;
            timer.start_instant = None;
            // Reset to full countdown duration
            timer.elapsed_ms = timer.countdown_duration * 1000;
        }
        self.timer_sound_played.remove(timer_id);
        self.notify_change();
    }

    /// Add a new alarm
    pub fn add_alarm(&mut self, alarm: AlarmConfig) {
        self.alarms.push(alarm);
        self.notify_change();
    }

    /// Remove an alarm by ID
    pub fn remove_alarm(&mut self, alarm_id: &str) {
        self.dismiss_alarm(alarm_id);
        self.alarms.retain(|a| a.id != alarm_id);
        self.notify_change();
    }

    /// Update an alarm's configuration
    pub fn update_alarm(&mut self, alarm_id: &str, update_fn: impl FnOnce(&mut AlarmConfig)) {
        if let Some(alarm) = self.alarms.iter_mut().find(|a| a.id == alarm_id) {
            update_fn(alarm);
            self.notify_change();
        }
    }

    /// Dismiss a specific alarm by ID
    pub fn dismiss_alarm(&mut self, alarm_id: &str) {
        self.triggered_alarms.remove(alarm_id);
        self.alarm_sound_played.remove(alarm_id);
        self.last_alarm_check.remove(alarm_id);
        self.notify_change();
    }

    /// Dismiss all triggered alarms
    pub fn dismiss_all_alarms(&mut self) {
        let ids: Vec<_> = self.triggered_alarms.iter().cloned().collect();
        for id in ids {
            self.dismiss_alarm(&id);
        }
    }

    /// Dismiss all finished timers (reset them to stopped state)
    pub fn dismiss_finished_timers(&mut self) {
        let finished_ids: Vec<_> = self
            .timers
            .iter()
            .filter(|t| t.state == TimerState::Finished)
            .map(|t| t.id.clone())
            .collect();
        for id in finished_ids {
            self.stop_timer(&id);
        }
    }

    /// Check if any alarm or timer needs attention (visual cue)
    pub fn needs_attention(&self) -> bool {
        !self.triggered_alarms.is_empty()
            || self.timers.iter().any(|t| t.state == TimerState::Finished)
    }

    /// Check if any timer is finished
    pub fn any_timer_finished(&self) -> bool {
        self.timers.iter().any(|t| t.state == TimerState::Finished)
    }

    /// Check if any alarm is triggered
    pub fn any_alarm_triggered(&self) -> bool {
        !self.triggered_alarms.is_empty()
    }

    /// Get the most relevant timer for display
    pub fn get_display_timer(&self) -> Option<&TimerConfig> {
        // Priority: finished > running > paused > first timer
        self.timers
            .iter()
            .find(|t| t.state == TimerState::Finished)
            .or_else(|| self.timers.iter().find(|t| t.state == TimerState::Running))
            .or_else(|| self.timers.iter().find(|t| t.state == TimerState::Paused))
            .or_else(|| self.timers.first())
    }

    /// Update all timer states (call from main update loop)
    pub fn update(&mut self, hour: u32, minute: u32, second: u32, day_of_week: u32) {
        // Periodic diagnostic logging (every ~60 seconds based on typical 1s update interval)
        static UPDATE_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let count = UPDATE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count > 0 && count.is_multiple_of(60) {
            log::info!(
                "TimerManager diagnostics: {} callbacks, {} timers, {} alarms",
                self.change_callbacks.len(),
                self.timers.len(),
                self.alarms.len()
            );
        }

        // Periodic cache clearing to prevent memory leaks (every ~5 minutes)
        // This helps prevent gradual memory growth from internal caches in Pango, Cairo, etc.
        if count > 0 && count.is_multiple_of(300) {
            log::info!("Performing periodic cache maintenance");
            trigger_cache_maintenance();
        }

        self.update_timers();
        self.check_alarms(hour, minute, second, day_of_week);
        self.update_next_alarm(hour, minute, day_of_week);
    }

    fn update_timers(&mut self) {
        let mut finished_timer_ids: Vec<String> = Vec::new();

        for timer in &mut self.timers {
            match timer.state {
                TimerState::Running => {
                    if let Some(start) = timer.start_instant {
                        let elapsed_since_start = start.elapsed().as_millis() as u64;
                        // All timers are countdown timers - use saturating_sub for safety
                        let remaining = timer.elapsed_ms.saturating_sub(elapsed_since_start);
                        if remaining > 0 {
                            timer.elapsed_ms = remaining;
                            timer.start_instant = Some(Instant::now());
                        } else {
                            timer.elapsed_ms = 0;
                            timer.state = TimerState::Finished;
                            timer.start_instant = None;
                            finished_timer_ids.push(timer.id.clone());
                        }
                    }
                }
                TimerState::Stopped => {
                    // Keep countdown at initial duration when stopped
                    timer.elapsed_ms = timer.countdown_duration * 1000;
                }
                _ => {}
            }
        }

        // Play global timer sound for newly finished timers
        for timer_id in finished_timer_ids {
            if !self.timer_sound_played.contains(&timer_id) {
                Self::play_sound(&self.global_timer_sound);
                self.timer_sound_played.insert(timer_id);
            }
        }
    }

    fn check_alarms(&mut self, hour: u32, minute: u32, second: u32, day_of_week: u32) {
        let current = (hour, minute, second);

        // First pass: collect indices and minimal data (no sound clone)
        // Store: (index, alarm_id, alarm_time, day_matches)
        let alarm_checks: Vec<_> = self
            .alarms
            .iter()
            .enumerate()
            .filter(|(_, a)| a.enabled)
            .map(|(idx, alarm)| {
                let alarm_time = (alarm.hour, alarm.minute, alarm.second);
                let day_matches = alarm.days.is_empty() || alarm.days.contains(&day_of_week);
                (idx, alarm.id.clone(), alarm_time, day_matches)
            })
            .collect();

        // Collect indices of alarms that need sound played (rare case)
        let mut play_sound_indices: Vec<usize> = Vec::new();

        for (idx, alarm_id, alarm_time, day_matches) in alarm_checks {
            let last_check = self.last_alarm_check.get(&alarm_id).cloned();

            if day_matches && current == alarm_time {
                // Only trigger once per second
                if last_check != Some(current) {
                    self.triggered_alarms.insert(alarm_id.clone());
                    self.last_alarm_check.insert(alarm_id.clone(), current);

                    // Mark for sound playback if not already played
                    if !self.alarm_sound_played.contains(&alarm_id) {
                        play_sound_indices.push(idx);
                        self.alarm_sound_played.insert(alarm_id);
                    }
                }
            } else if last_check.is_some() && last_check != Some(current) {
                // Time has passed - auto re-arm for repeating alarms
                self.triggered_alarms.remove(&alarm_id);
                self.alarm_sound_played.remove(&alarm_id);
                self.last_alarm_check.remove(&alarm_id);
            }
        }

        // Play sounds for newly triggered alarms (only clone sound when actually needed)
        for idx in play_sound_indices {
            if let Some(alarm) = self.alarms.get(idx) {
                Self::play_sound(&alarm.sound);
            }
        }
    }

    fn update_next_alarm(&mut self, current_hour: u32, current_minute: u32, day_of_week: u32) {
        let mut next_alarm: Option<(&AlarmConfig, u64)> = None;

        for alarm in self.alarms.iter().filter(|a| a.enabled) {
            let minutes_until =
                Self::minutes_until_alarm(alarm, current_hour, current_minute, day_of_week);
            if let Some((_, current_min)) = next_alarm {
                if minutes_until < current_min {
                    next_alarm = Some((alarm, minutes_until));
                }
            } else {
                next_alarm = Some((alarm, minutes_until));
            }
        }

        if let Some((alarm, _)) = next_alarm {
            self.next_alarm_time = Some(format!("{:02}:{:02}", alarm.hour, alarm.minute));
            self.next_alarm_id = Some(alarm.id.clone());
        } else {
            self.next_alarm_time = None;
            self.next_alarm_id = None;
        }
    }

    fn minutes_until_alarm(
        alarm: &AlarmConfig,
        current_hour: u32,
        current_minute: u32,
        day_of_week: u32,
    ) -> u64 {
        let current_minutes = (current_hour * 60 + current_minute) as i64;
        let alarm_minutes = (alarm.hour * 60 + alarm.minute) as i64;

        let mut diff = alarm_minutes - current_minutes;

        if diff <= 0 || (!alarm.days.is_empty() && !alarm.days.contains(&day_of_week)) {
            if alarm.days.is_empty() {
                diff += 24 * 60;
            } else {
                let mut days_until = 1u64;
                for i in 1..=7 {
                    let check_day = (day_of_week + i) % 7;
                    if alarm.days.contains(&check_day) {
                        days_until = i as u64;
                        break;
                    }
                }
                diff = (days_until as i64 * 24 * 60) + (alarm_minutes - current_minutes);
                if diff < 0 {
                    diff += 24 * 60;
                }
            }
        }

        diff.max(0) as u64
    }

    fn play_sound(sound_config: &AlarmSoundConfig) {
        if !sound_config.enabled {
            return;
        }

        // Use the global audio thread so sounds can be stopped
        play_preview_sound(sound_config);
    }

    /// Get serializable config for saving
    pub fn get_config(&self) -> (Vec<TimerConfig>, Vec<AlarmConfig>) {
        (self.timers.clone(), self.alarms.clone())
    }

    /// Get full config including global timer sound
    pub fn get_full_config(&self) -> (Vec<TimerConfig>, Vec<AlarmConfig>, AlarmSoundConfig) {
        (
            self.timers.clone(),
            self.alarms.clone(),
            self.global_timer_sound.clone(),
        )
    }
}

impl Default for TimerAlarmManager {
    fn default() -> Self {
        Self::new()
    }
}
