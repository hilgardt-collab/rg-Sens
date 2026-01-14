//! Synthwave/Outrun Displayer
//!
//! A retro-futuristic 80s aesthetic with:
//! - Purple/pink/cyan gradient backgrounds
//! - Neon grid lines (classic 80s grid horizon)
//! - Chrome/metallic text effects
//! - Sunset gradient accents
//! - Retro-futuristic fonts

use anyhow::Result;
use cairo::Context;
use gtk4::Widget;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig};
use crate::displayers::combo_generic::GenericComboDisplayerShared;
use crate::ui::synthwave_display::{SynthwaveFrameConfig, SynthwaveRenderer};

/// Full Synthwave display configuration (wrapper for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthwaveDisplayConfig {
    #[serde(default)]
    pub frame: SynthwaveFrameConfig,
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

impl Default for SynthwaveDisplayConfig {
    fn default() -> Self {
        Self {
            frame: SynthwaveFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

impl SynthwaveDisplayConfig {
    pub fn from_frame(frame: SynthwaveFrameConfig) -> Self {
        Self {
            animation_enabled: frame.animation_enabled,
            animation_speed: frame.animation_speed,
            frame,
        }
    }

    pub fn to_frame(&self) -> SynthwaveFrameConfig {
        let mut frame = self.frame.clone();
        frame.animation_enabled = self.animation_enabled;
        frame.animation_speed = self.animation_speed;
        frame
    }
}

/// Synthwave/Outrun Displayer
pub struct SynthwaveDisplayer {
    inner: GenericComboDisplayerShared<SynthwaveRenderer>,
}

impl SynthwaveDisplayer {
    pub fn new() -> Self {
        Self {
            inner: GenericComboDisplayerShared::new(SynthwaveRenderer),
        }
    }
}

impl Default for SynthwaveDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for SynthwaveDisplayer {
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
                    key: "color_scheme".to_string(),
                    name: "Color Scheme".to_string(),
                    description: "Synthwave color palette".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("sunset"),
                },
                ConfigOption {
                    key: "grid_enabled".to_string(),
                    name: "Grid Lines".to_string(),
                    description: "Enable retro grid horizon".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
                ConfigOption {
                    key: "scanline_effect".to_string(),
                    name: "Scanline Effect".to_string(),
                    description: "Enable CRT scanline effect".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(false),
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
        if let Some(config_value) = config.get("synthwave_config") {
            if let Ok(display_config) =
                serde_json::from_value::<SynthwaveDisplayConfig>(config_value.clone())
            {
                self.inner.set_config(display_config.to_frame());
                return Ok(());
            }
            if let Ok(frame_config) =
                serde_json::from_value::<SynthwaveFrameConfig>(config_value.clone())
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
        self.inner
            .get_config()
            .map(|frame| DisplayerConfig::Synthwave(SynthwaveDisplayConfig::from_frame(frame)))
    }
}
