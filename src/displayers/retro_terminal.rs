//! Retro Terminal (CRT) Displayer
//!
//! A vintage CRT terminal aesthetic with:
//! - Green or amber phosphor text on dark background
//! - CRT scanline and curvature effects
//! - Monitor bezel frame styling
//! - Phosphor glow (screen burn) around bright elements

use anyhow::Result;
use cairo::Context;
use gtk4::Widget;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig};
use crate::displayers::combo_generic::GenericComboDisplayerShared;
use crate::ui::retro_terminal_display::{RetroTerminalFrameConfig, RetroTerminalRenderer};

/// Full Retro Terminal display configuration (wrapper for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetroTerminalDisplayConfig {
    #[serde(default)]
    pub frame: RetroTerminalFrameConfig,
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
}

fn default_animation_enabled() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    8.0
}

impl Default for RetroTerminalDisplayConfig {
    fn default() -> Self {
        Self {
            frame: RetroTerminalFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

impl RetroTerminalDisplayConfig {
    pub fn from_frame(frame: RetroTerminalFrameConfig) -> Self {
        Self {
            animation_enabled: frame.animation_enabled,
            animation_speed: frame.animation_speed,
            frame,
        }
    }

    pub fn to_frame(&self) -> RetroTerminalFrameConfig {
        let mut frame = self.frame.clone();
        frame.animation_enabled = self.animation_enabled;
        frame.animation_speed = self.animation_speed;
        frame
    }
}

/// Retro Terminal (CRT) Displayer
pub struct RetroTerminalDisplayer {
    inner: GenericComboDisplayerShared<RetroTerminalRenderer>,
}

impl RetroTerminalDisplayer {
    pub fn new() -> Self {
        Self {
            inner: GenericComboDisplayerShared::new(RetroTerminalRenderer),
        }
    }
}

impl Default for RetroTerminalDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for RetroTerminalDisplayer {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn create_widget(&self) -> Widget {
        self.inner.create_widget()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        self.inner.update_data(data)
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        self.inner.draw(cr, width, height)
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "phosphor_color".to_string(),
                    name: "Phosphor Color".to_string(),
                    description: "Terminal phosphor color preset".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("green"),
                },
                ConfigOption {
                    key: "scanline_intensity".to_string(),
                    name: "Scanline Intensity".to_string(),
                    description: "Intensity of CRT scanline effect".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(0.25),
                },
                ConfigOption {
                    key: "screen_glow".to_string(),
                    name: "Screen Glow".to_string(),
                    description: "Phosphor glow/bloom intensity".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(0.5),
                },
                ConfigOption {
                    key: "animation_enabled".to_string(),
                    name: "Animation".to_string(),
                    description: "Enable smooth animations".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        if let Some(config_value) = config.get("retro_terminal_config") {
            if let Ok(display_config) =
                serde_json::from_value::<RetroTerminalDisplayConfig>(config_value.clone())
            {
                self.inner.set_config(display_config.to_frame());
                return Ok(());
            }
            if let Ok(frame_config) =
                serde_json::from_value::<RetroTerminalFrameConfig>(config_value.clone())
            {
                self.inner.set_config(frame_config);
                return Ok(());
            }
        }
        self.inner.apply_config(config)
    }

    fn needs_redraw(&self) -> bool {
        self.inner.needs_redraw()
    }

    fn get_typed_config(&self) -> Option<DisplayerConfig> {
        self.inner.get_config().map(|frame| {
            DisplayerConfig::RetroTerminal(RetroTerminalDisplayConfig::from_frame(frame))
        })
    }
}
