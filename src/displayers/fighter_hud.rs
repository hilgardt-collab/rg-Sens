//! Fighter Jet HUD Displayer
//!
//! A military fighter jet heads-up display aesthetic with:
//! - Military green/amber monochrome color scheme
//! - Thin line frames with corner brackets [ ]
//! - Targeting reticle aesthetics for gauges
//! - Altitude/heading ladder-style scales
//! - Stencil military font styling

use anyhow::Result;
use cairo::Context;
use gtk4::Widget;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig};
use crate::displayers::combo_generic::GenericComboDisplayerShared;
use crate::ui::fighter_hud_display::{FighterHudFrameConfig, FighterHudRenderer};

/// Full Fighter HUD display configuration (wrapper for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FighterHudDisplayConfig {
    #[serde(default)]
    pub frame: FighterHudFrameConfig,
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

impl Default for FighterHudDisplayConfig {
    fn default() -> Self {
        Self {
            frame: FighterHudFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

impl FighterHudDisplayConfig {
    pub fn from_frame(frame: FighterHudFrameConfig) -> Self {
        Self {
            animation_enabled: frame.animation_enabled,
            animation_speed: frame.animation_speed,
            frame,
        }
    }

    pub fn to_frame(&self) -> FighterHudFrameConfig {
        let mut frame = self.frame.clone();
        frame.animation_enabled = self.animation_enabled;
        frame.animation_speed = self.animation_speed;
        frame
    }
}

/// Fighter Jet HUD Displayer
pub struct FighterHudDisplayer {
    inner: GenericComboDisplayerShared<FighterHudRenderer>,
}

impl FighterHudDisplayer {
    pub fn new() -> Self {
        Self {
            inner: GenericComboDisplayerShared::new(FighterHudRenderer),
        }
    }
}

impl Default for FighterHudDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for FighterHudDisplayer {
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
                    key: "hud_color".to_string(),
                    name: "HUD Color".to_string(),
                    description: "HUD color preset".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("military_green"),
                },
                ConfigOption {
                    key: "frame_style".to_string(),
                    name: "Frame Style".to_string(),
                    description: "Style of the HUD frame corners".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("corner_brackets"),
                },
                ConfigOption {
                    key: "glow_intensity".to_string(),
                    name: "Glow Intensity".to_string(),
                    description: "Intensity of the HUD glow effect".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(0.3),
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
        if let Some(config_value) = config.get("fighter_hud_config") {
            if let Ok(display_config) = serde_json::from_value::<FighterHudDisplayConfig>(config_value.clone()) {
                self.inner.set_config(display_config.to_frame());
                return Ok(());
            }
            if let Ok(frame_config) = serde_json::from_value::<FighterHudFrameConfig>(config_value.clone()) {
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
            DisplayerConfig::FighterHud(FighterHudDisplayConfig::from_frame(frame))
        })
    }
}
