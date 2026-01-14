//! Industrial/Gauge Panel displayer
//!
//! Visualizes combo source data with industrial aesthetic:
//! - Brushed metal/carbon fiber textures
//! - Physical gauge aesthetics (rivets, bezels)
//! - Warning stripe accents
//! - Heavy bold typography

use anyhow::Result;
use cairo::Context;
use gtk4::Widget;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig};
use crate::displayers::combo_generic::GenericComboDisplayerShared;
use crate::ui::industrial_display::{IndustrialFrameConfig, IndustrialRenderer};

/// Industrial display configuration (wrapper for backward compatibility)
///
/// This struct maintains backward compatibility with saved configs that have
/// the animation fields at the top level alongside a `frame` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustrialDisplayConfig {
    pub frame: IndustrialFrameConfig,
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

impl Default for IndustrialDisplayConfig {
    fn default() -> Self {
        Self {
            frame: IndustrialFrameConfig::default(),
            animation_enabled: true,
            animation_speed: 8.0,
        }
    }
}

impl IndustrialDisplayConfig {
    /// Create config from frame config, syncing animation fields
    pub fn from_frame(frame: IndustrialFrameConfig) -> Self {
        Self {
            animation_enabled: frame.animation_enabled,
            animation_speed: frame.animation_speed,
            frame,
        }
    }

    /// Convert to frame config, syncing animation fields from wrapper
    pub fn to_frame(&self) -> IndustrialFrameConfig {
        let mut frame = self.frame.clone();
        frame.animation_enabled = self.animation_enabled;
        frame.animation_speed = self.animation_speed;
        frame
    }
}

/// Industrial/Gauge Panel displayer
pub struct IndustrialDisplayer {
    inner: GenericComboDisplayerShared<IndustrialRenderer>,
}

impl IndustrialDisplayer {
    pub fn new() -> Self {
        Self {
            inner: GenericComboDisplayerShared::new(IndustrialRenderer),
        }
    }
}

impl Default for IndustrialDisplayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Displayer for IndustrialDisplayer {
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
                    key: "surface_texture".to_string(),
                    name: "Surface Texture".to_string(),
                    description: "Metal surface texture style".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("brushed_metal"),
                },
                ConfigOption {
                    key: "rivet_style".to_string(),
                    name: "Rivet Style".to_string(),
                    description: "Style of rivets/bolts".to_string(),
                    value_type: "string".to_string(),
                    default: serde_json::json!("hex"),
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
        // Check for full industrial_config first (wrapper format)
        if let Some(config_value) = config.get("industrial_config") {
            if let Ok(display_config) =
                serde_json::from_value::<IndustrialDisplayConfig>(config_value.clone())
            {
                // Convert wrapper to frame config and apply
                self.inner.set_config(display_config.to_frame());
                return Ok(());
            }
            // Try direct IndustrialFrameConfig (new format)
            if let Ok(frame_config) =
                serde_json::from_value::<IndustrialFrameConfig>(config_value.clone())
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
            .map(|frame| DisplayerConfig::Industrial(IndustrialDisplayConfig::from_frame(frame)))
    }
}
