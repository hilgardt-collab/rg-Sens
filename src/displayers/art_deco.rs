//! Art Deco Displayer
//!
//! A 1920s-inspired Art Deco display with:
//! - Sunburst and fan corner decorations
//! - Stepped/ziggurat border patterns
//! - Chevron dividers and accents
//! - Gold, copper, brass metallic color schemes
//! - Support for multiple data source groups

use anyhow::Result;
use cairo::Context;
use gtk4::Widget;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig};
use crate::displayers::combo_generic::GenericComboDisplayerShared;
use crate::ui::art_deco_display::{ArtDecoFrameConfig, ArtDecoRenderer};

/// Full Art Deco display configuration (wrapper for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtDecoDisplayConfig {
    #[serde(default)]
    pub frame: ArtDecoFrameConfig,
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

impl Default for ArtDecoDisplayConfig {
    fn default() -> Self {
        Self {
            frame: ArtDecoFrameConfig::default(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
        }
    }
}

impl ArtDecoDisplayConfig {
    /// Create config from frame config, syncing animation fields
    pub fn from_frame(frame: ArtDecoFrameConfig) -> Self {
        Self {
            animation_enabled: frame.animation_enabled,
            animation_speed: frame.animation_speed,
            frame,
        }
    }

    /// Convert to frame config, syncing animation fields from wrapper
    pub fn to_frame(&self) -> ArtDecoFrameConfig {
        let mut frame = self.frame.clone();
        frame.animation_enabled = self.animation_enabled;
        frame.animation_speed = self.animation_speed;
        frame
    }
}

/// Art Deco Displayer
pub struct ArtDecoDisplayer {
    inner: GenericComboDisplayerShared<ArtDecoRenderer>,
}

impl ArtDecoDisplayer {
    pub fn new() -> Self {
        Self {
            inner: GenericComboDisplayerShared::new(ArtDecoRenderer),
        }
    }
}

impl Default for ArtDecoDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for ArtDecoDisplayer {
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
                    description: "Art Deco metallic color palette".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("gold"),
                },
                ConfigOption {
                    key: "corner_style".to_string(),
                    name: "Corner Style".to_string(),
                    description: "Style of corner decorations".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("sunburst"),
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
        // Check for full art_deco_config first (wrapper format)
        if let Some(config_value) = config.get("art_deco_config") {
            if let Ok(display_config) =
                serde_json::from_value::<ArtDecoDisplayConfig>(config_value.clone())
            {
                self.inner.set_config(display_config.to_frame());
                return Ok(());
            }
            // Try direct ArtDecoFrameConfig (new format)
            if let Ok(frame_config) =
                serde_json::from_value::<ArtDecoFrameConfig>(config_value.clone())
            {
                self.inner.set_config(frame_config);
                return Ok(());
            }
        }

        // Delegate to inner for individual field updates
        self.inner.apply_config(config)
    }

    fn needs_redraw(&self) -> bool {
        self.inner.needs_redraw()
    }

    fn get_typed_config(&self) -> Option<DisplayerConfig> {
        self.inner
            .get_config()
            .map(|frame| DisplayerConfig::ArtDeco(ArtDecoDisplayConfig::from_frame(frame)))
    }
}
