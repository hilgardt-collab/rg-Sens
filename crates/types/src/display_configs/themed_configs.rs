//! Themed display configuration wrappers
//!
//! Each themed DisplayConfig wraps a FrameConfig and adds animation settings.
//! These are the top-level types stored in `DisplayerConfig` enum variants.

use serde::{Deserialize, Serialize};

use super::art_deco::ArtDecoFrameConfig;
use super::art_nouveau::ArtNouveauFrameConfig;
use super::cyberpunk::CyberpunkFrameConfig;
use super::fighter_hud::FighterHudFrameConfig;
use super::industrial::IndustrialFrameConfig;
use super::lcars::LcarsFrameConfig;
use super::material::MaterialFrameConfig;
use super::retro_terminal::RetroTerminalFrameConfig;
use super::steampunk::SteampunkFrameConfig;
use super::synthwave::SynthwaveFrameConfig;

fn default_animation_enabled() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    8.0
}

/// Generate a themed display config wrapper struct.
///
/// Each struct has: frame (FrameConfig), animation_enabled (bool), animation_speed (f64)
/// Plus Default, from_frame(), and to_frame() implementations.
macro_rules! themed_display_config {
    ($config_name:ident, $frame_config:ty) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct $config_name {
            #[serde(default)]
            pub frame: $frame_config,
            #[serde(default = "default_animation_enabled")]
            pub animation_enabled: bool,
            #[serde(default = "default_animation_speed")]
            pub animation_speed: f64,
        }

        impl Default for $config_name {
            fn default() -> Self {
                Self {
                    frame: <$frame_config>::default(),
                    animation_enabled: default_animation_enabled(),
                    animation_speed: default_animation_speed(),
                }
            }
        }

        impl $config_name {
            /// Create config from frame config, syncing animation fields
            pub fn from_frame(frame: $frame_config) -> Self {
                Self {
                    animation_enabled: frame.animation_enabled,
                    animation_speed: frame.animation_speed,
                    frame,
                }
            }

            /// Convert to frame config, syncing animation fields from wrapper
            pub fn to_frame(&self) -> $frame_config {
                let mut frame = self.frame.clone();
                frame.animation_enabled = self.animation_enabled;
                frame.animation_speed = self.animation_speed;
                frame
            }
        }
    };
}

/// Full LCARS display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LcarsDisplayConfig {
    /// Frame and sidebar configuration
    #[serde(default)]
    pub frame: LcarsFrameConfig,

    /// Animation settings
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
}

impl Default for LcarsDisplayConfig {
    fn default() -> Self {
        Self {
            frame: LcarsFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

themed_display_config!(CyberpunkDisplayConfig, CyberpunkFrameConfig);
themed_display_config!(MaterialDisplayConfig, MaterialFrameConfig);
themed_display_config!(IndustrialDisplayConfig, IndustrialFrameConfig);
themed_display_config!(RetroTerminalDisplayConfig, RetroTerminalFrameConfig);
themed_display_config!(FighterHudDisplayConfig, FighterHudFrameConfig);
themed_display_config!(SynthwaveDisplayConfig, SynthwaveFrameConfig);
themed_display_config!(ArtDecoDisplayConfig, ArtDecoFrameConfig);
themed_display_config!(ArtNouveauDisplayConfig, ArtNouveauFrameConfig);
themed_display_config!(SteampunkDisplayConfig, SteampunkFrameConfig);
