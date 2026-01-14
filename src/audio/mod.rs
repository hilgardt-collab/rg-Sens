//! Audio playback module for alarm and timer sounds

use anyhow::{Context, Result};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Audio player that can play, stop, and control volume of alarm sounds
pub struct AudioPlayer {
    // Keep the stream alive - dropping it stops all audio
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    sink: Arc<Mutex<Sink>>,
}

impl AudioPlayer {
    /// Create a new audio player
    pub fn new() -> Result<Self> {
        let (stream, stream_handle) =
            OutputStream::try_default().context("Failed to open audio output stream")?;
        let sink = Sink::try_new(&stream_handle).context("Failed to create audio sink")?;

        Ok(Self {
            _stream: stream,
            _stream_handle: stream_handle,
            sink: Arc::new(Mutex::new(sink)),
        })
    }

    /// Play a sound file from the given path
    pub fn play(&self, path: &str) -> Result<()> {
        let file =
            File::open(path).with_context(|| format!("Failed to open sound file: {}", path))?;
        let source = Decoder::new(BufReader::new(file))
            .with_context(|| format!("Failed to decode sound file: {}", path))?;

        let sink = self.sink.lock().unwrap_or_else(|e| e.into_inner());
        sink.append(source);
        sink.play();
        Ok(())
    }

    /// Play a sound file in a loop
    pub fn play_looped(&self, path: &str) -> Result<()> {
        let file =
            File::open(path).with_context(|| format!("Failed to open sound file: {}", path))?;
        let source = Decoder::new(BufReader::new(file))
            .with_context(|| format!("Failed to decode sound file: {}", path))?
            .repeat_infinite();

        let sink = self.sink.lock().unwrap_or_else(|e| e.into_inner());
        sink.append(source);
        sink.play();
        Ok(())
    }

    /// Try to play a system alert sound
    pub fn play_system_alert(&self) -> Result<()> {
        // Common system alert sound paths
        let paths = [
            // freedesktop sounds (Linux)
            "/usr/share/sounds/freedesktop/stereo/alarm-clock-elapsed.oga",
            "/usr/share/sounds/freedesktop/stereo/complete.oga",
            "/usr/share/sounds/freedesktop/stereo/bell.oga",
            "/usr/share/sounds/freedesktop/stereo/message.oga",
            // Ubuntu/GNOME sounds
            "/usr/share/sounds/gnome/default/alerts/drip.ogg",
            "/usr/share/sounds/gnome/default/alerts/glass.ogg",
            // KDE sounds
            "/usr/share/sounds/Oxygen-Sys-App-Message.ogg",
            // macOS
            "/System/Library/Sounds/Glass.aiff",
            "/System/Library/Sounds/Ping.aiff",
            // Windows
            "C:\\Windows\\Media\\Alarm01.wav",
            "C:\\Windows\\Media\\notify.wav",
        ];

        for path in paths {
            if std::path::Path::new(path).exists() && self.play(path).is_ok() {
                return Ok(());
            }
        }

        // Fallback: Generate a simple tone
        self.play_beep(440.0, Duration::from_millis(500))
    }

    /// Play a simple beep tone at the given frequency and duration
    pub fn play_beep(&self, frequency: f32, duration: Duration) -> Result<()> {
        let sample_rate = 44100u32;
        let _num_samples = (sample_rate as f64 * duration.as_secs_f64()) as usize;

        let source = rodio::source::SineWave::new(frequency)
            .take_duration(duration)
            .amplify(0.3); // Reduce volume to avoid being too loud

        let sink = self.sink.lock().unwrap_or_else(|e| e.into_inner());
        sink.append(source);
        sink.play();
        Ok(())
    }

    /// Stop the currently playing sound
    pub fn stop(&self) {
        let sink = self.sink.lock().unwrap_or_else(|e| e.into_inner());
        sink.stop();
    }

    /// Set the volume (0.0 to 1.0)
    pub fn set_volume(&self, volume: f32) {
        let sink = self.sink.lock().unwrap_or_else(|e| e.into_inner());
        sink.set_volume(volume.clamp(0.0, 1.0));
    }

    /// Check if audio is currently playing
    pub fn is_playing(&self) -> bool {
        // Return false if lock is poisoned - safer than panicking
        if let Ok(sink) = self.sink.lock() {
            !sink.empty()
        } else {
            false
        }
    }

    /// Pause playback
    pub fn pause(&self) {
        let sink = self.sink.lock().unwrap_or_else(|e| e.into_inner());
        sink.pause();
    }

    /// Resume playback
    pub fn resume(&self) {
        let sink = self.sink.lock().unwrap_or_else(|e| e.into_inner());
        sink.play();
    }
}

/// Configuration for alarm/timer sounds
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AlarmSoundConfig {
    /// Whether sound is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Custom sound file path (None = use system alert)
    #[serde(default)]
    pub custom_sound_path: Option<String>,

    /// Whether to loop the sound until dismissed
    #[serde(default = "default_true")]
    pub loop_sound: bool,

    /// Volume level (0.0 to 1.0)
    #[serde(default = "default_volume")]
    pub volume: f32,

    /// Whether visual flash effect is enabled
    #[serde(default = "default_true")]
    pub visual_enabled: bool,
}

fn default_true() -> bool {
    true
}

fn default_volume() -> f32 {
    0.8
}

impl Default for AlarmSoundConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            custom_sound_path: None,
            loop_sound: true,
            volume: 0.8,
            visual_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_player_creation() {
        // This may fail in CI environments without audio
        let result = AudioPlayer::new();
        // Just check it doesn't panic - may fail on systems without audio
        if result.is_err() {
            eprintln!(
                "Audio player creation failed (expected in CI): {:?}",
                result.err()
            );
        }
    }

    #[test]
    fn test_alarm_sound_config_default() {
        let config = AlarmSoundConfig::default();
        assert!(config.enabled);
        assert!(config.custom_sound_path.is_none());
        assert!(config.loop_sound);
        assert!((config.volume - 0.8).abs() < 0.001);
    }
}
