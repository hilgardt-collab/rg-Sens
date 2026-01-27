//! Material Design Cards Displayer
//!
//! A clean, modern Material Design-inspired interface with:
//! - Clean white/dark cards with subtle shadows
//! - Large rounded corners
//! - Generous whitespace and padding
//! - Color-coded category headers
//! - Support for multiple data source groups

use anyhow::Result;
use cairo::Context;
use gtk4::Widget;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig};
use crate::ui::material_display::{MaterialFrameConfig, MaterialRenderer};

// Use shared animation defaults from parent module
use super::{default_animation_enabled, default_animation_speed};

/// Full Material display configuration (wrapper for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDisplayConfig {
    #[serde(default)]
    pub frame: MaterialFrameConfig,
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
}

impl Default for MaterialDisplayConfig {
    fn default() -> Self {
        Self {
            frame: MaterialFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

impl MaterialDisplayConfig {
    pub fn from_frame(frame: MaterialFrameConfig) -> Self {
        Self {
            animation_enabled: frame.animation_enabled,
            animation_speed: frame.animation_speed,
            frame,
        }
    }

    pub fn to_frame(&self) -> MaterialFrameConfig {
        let mut frame = self.frame.clone();
        frame.animation_enabled = self.animation_enabled;
        frame.animation_speed = self.animation_speed;
        frame
    }
}

// Use macro to generate displayer struct and basic implementations
crate::theme_displayer_base!(MaterialDisplayer, MaterialRenderer, MaterialRenderer);

impl Displayer for MaterialDisplayer {
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
                    key: "theme".to_string(),
                    name: "Theme".to_string(),
                    description: "Light or dark theme".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("light"),
                },
                ConfigOption {
                    key: "accent_color".to_string(),
                    name: "Accent Color".to_string(),
                    description: "Primary accent color".to_string(),
                    value_type: "color".to_string(),
                    default: serde_json::json!([0.24, 0.47, 0.96, 1.0]),
                },
                ConfigOption {
                    key: "elevation".to_string(),
                    name: "Card Elevation".to_string(),
                    description: "Shadow depth of cards".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("low"),
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
        if let Some(config_value) = config.get("material_config") {
            if let Ok(display_config) =
                serde_json::from_value::<MaterialDisplayConfig>(config_value.clone())
            {
                self.inner.set_config(display_config.to_frame());
                return Ok(());
            }
            if let Ok(frame_config) =
                serde_json::from_value::<MaterialFrameConfig>(config_value.clone())
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
            .map(|frame| DisplayerConfig::Material(MaterialDisplayConfig::from_frame(frame)))
    }
}
