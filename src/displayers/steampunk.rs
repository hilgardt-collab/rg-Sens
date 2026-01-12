//! Steampunk Displayer
//!
//! A Victorian-era steampunk display with:
//! - Brass, copper, and bronze metallic colors
//! - Decorative gears and cogs
//! - Ornate rivets and Victorian flourishes
//! - Steam pipe and gauge aesthetics
//! - Weathered patina textures
//! - Support for multiple data source groups

use anyhow::Result;
use cairo::Context;
use gtk4::Widget;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig};
use crate::displayers::combo_generic::GenericComboDisplayerShared;
use crate::ui::steampunk_display::{SteampunkFrameConfig, SteampunkRenderer};

/// Full Steampunk display configuration (wrapper for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteampunkDisplayConfig {
    #[serde(default)]
    pub frame: SteampunkFrameConfig,
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

impl Default for SteampunkDisplayConfig {
    fn default() -> Self {
        Self {
            frame: SteampunkFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

impl SteampunkDisplayConfig {
    pub fn from_frame(frame: SteampunkFrameConfig) -> Self {
        Self {
            animation_enabled: frame.animation_enabled,
            animation_speed: frame.animation_speed,
            frame,
        }
    }

    pub fn to_frame(&self) -> SteampunkFrameConfig {
        let mut frame = self.frame.clone();
        frame.animation_enabled = self.animation_enabled;
        frame.animation_speed = self.animation_speed;
        frame
    }
}

/// Steampunk Displayer
pub struct SteampunkDisplayer {
    inner: GenericComboDisplayerShared<SteampunkRenderer>,
}

impl SteampunkDisplayer {
    pub fn new() -> Self {
        Self {
            inner: GenericComboDisplayerShared::new(SteampunkRenderer),
        }
    }
}

impl Default for SteampunkDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for SteampunkDisplayer {
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
                    key: "border_style".to_string(),
                    name: "Border Style".to_string(),
                    description: "Style of frame border".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("victorian"),
                },
                ConfigOption {
                    key: "corner_style".to_string(),
                    name: "Corner Style".to_string(),
                    description: "Style of corner decorations".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("gear"),
                },
                ConfigOption {
                    key: "background_texture".to_string(),
                    name: "Background Texture".to_string(),
                    description: "Background texture style".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("brushed_brass"),
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
        if let Some(config_value) = config.get("steampunk_config") {
            if let Ok(display_config) = serde_json::from_value::<SteampunkDisplayConfig>(config_value.clone()) {
                self.inner.set_config(display_config.to_frame());
                return Ok(());
            }
            if let Ok(frame_config) = serde_json::from_value::<SteampunkFrameConfig>(config_value.clone()) {
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
            DisplayerConfig::Steampunk(SteampunkDisplayConfig::from_frame(frame))
        })
    }
}
