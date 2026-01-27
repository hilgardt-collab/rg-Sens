//! Art Nouveau Displayer
//!
//! An organic, nature-inspired Art Nouveau display with:
//! - Flowing vine and whiplash curve borders
//! - Floral and leaf corner decorations
//! - Wave and tendril dividers
//! - Earthy color schemes (olive, gold, cream)
//! - Support for multiple data source groups

use anyhow::Result;
use cairo::Context;
use gtk4::Widget;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig};
use crate::ui::art_nouveau_display::{ArtNouveauFrameConfig, ArtNouveauRenderer};

// Use shared animation defaults from parent module
use super::{default_animation_enabled, default_animation_speed};

/// Full Art Nouveau display configuration (wrapper for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtNouveauDisplayConfig {
    #[serde(default)]
    pub frame: ArtNouveauFrameConfig,
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
}

impl Default for ArtNouveauDisplayConfig {
    fn default() -> Self {
        Self {
            frame: ArtNouveauFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

impl ArtNouveauDisplayConfig {
    pub fn from_frame(frame: ArtNouveauFrameConfig) -> Self {
        Self {
            animation_enabled: frame.animation_enabled,
            animation_speed: frame.animation_speed,
            frame,
        }
    }

    pub fn to_frame(&self) -> ArtNouveauFrameConfig {
        let mut frame = self.frame.clone();
        frame.animation_enabled = self.animation_enabled;
        frame.animation_speed = self.animation_speed;
        frame
    }
}

// Use macro to generate displayer struct and basic implementations
crate::theme_displayer_base!(ArtNouveauDisplayer, ArtNouveauRenderer, ArtNouveauRenderer);

impl Displayer for ArtNouveauDisplayer {
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
                    description: "Art Nouveau nature-inspired color palette".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("spring"),
                },
                ConfigOption {
                    key: "vine_style".to_string(),
                    name: "Vine Style".to_string(),
                    description: "Style of vine decorations".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("flowing"),
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
        if let Some(config_value) = config.get("art_nouveau_config") {
            if let Ok(display_config) =
                serde_json::from_value::<ArtNouveauDisplayConfig>(config_value.clone())
            {
                self.inner.set_config(display_config.to_frame());
                return Ok(());
            }
            if let Ok(frame_config) =
                serde_json::from_value::<ArtNouveauFrameConfig>(config_value.clone())
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
            .map(|frame| DisplayerConfig::ArtNouveau(ArtNouveauDisplayConfig::from_frame(frame)))
    }
}
