//! Cyberpunk/Neon HUD Displayer
//!
//! A futuristic heads-up display with:
//! - Angular chamfered corners with neon glow effects
//! - Dark translucent backgrounds with grid patterns
//! - Scanline overlay for CRT/hologram effect
//! - Support for multiple data source groups

use anyhow::Result;
use cairo::Context;
use gtk4::Widget;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig};
use crate::displayers::combo_generic::GenericComboDisplayerShared;
use crate::ui::cyberpunk_display::{CyberpunkFrameConfig, CyberpunkRenderer};

/// Full Cyberpunk display configuration (wrapper for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CyberpunkDisplayConfig {
    #[serde(default)]
    pub frame: CyberpunkFrameConfig,
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

impl Default for CyberpunkDisplayConfig {
    fn default() -> Self {
        Self {
            frame: CyberpunkFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

impl CyberpunkDisplayConfig {
    pub fn from_frame(frame: CyberpunkFrameConfig) -> Self {
        Self {
            animation_enabled: frame.animation_enabled,
            animation_speed: frame.animation_speed,
            frame,
        }
    }

    pub fn to_frame(&self) -> CyberpunkFrameConfig {
        let mut frame = self.frame.clone();
        frame.animation_enabled = self.animation_enabled;
        frame.animation_speed = self.animation_speed;
        frame
    }
}

/// Cyberpunk/Neon HUD Displayer
pub struct CyberpunkDisplayer {
    inner: GenericComboDisplayerShared<CyberpunkRenderer>,
}

impl CyberpunkDisplayer {
    pub fn new() -> Self {
        Self {
            inner: GenericComboDisplayerShared::new(CyberpunkRenderer),
        }
    }
}

impl Default for CyberpunkDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for CyberpunkDisplayer {
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
                    key: "border_color".to_string(),
                    name: "Border Color".to_string(),
                    description: "Neon border color".to_string(),
                    value_type: "color".to_string(),
                    default: serde_json::json!([0.0, 1.0, 1.0, 1.0]),
                },
                ConfigOption {
                    key: "glow_intensity".to_string(),
                    name: "Glow Intensity".to_string(),
                    description: "Intensity of the neon glow effect".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(0.6),
                },
                ConfigOption {
                    key: "show_scanlines".to_string(),
                    name: "Show Scanlines".to_string(),
                    description: "Enable CRT scanline effect".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
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
        if let Some(config_value) = config.get("cyberpunk_config") {
            if let Ok(display_config) =
                serde_json::from_value::<CyberpunkDisplayConfig>(config_value.clone())
            {
                log::debug!(
                    "CyberpunkDisplayer::apply_config - loaded {} groups, {} content_items",
                    display_config.frame.group_count,
                    display_config.frame.content_items.len()
                );
                self.inner.set_config(display_config.to_frame());
                return Ok(());
            }
            if let Ok(frame_config) =
                serde_json::from_value::<CyberpunkFrameConfig>(config_value.clone())
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
            log::debug!(
                "CyberpunkDisplayer::get_typed_config - saving {} groups, {} content_items",
                frame.group_count,
                frame.content_items.len()
            );
            DisplayerConfig::Cyberpunk(CyberpunkDisplayConfig::from_frame(frame))
        })
    }
}
