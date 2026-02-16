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
use serde_json::Value;
use std::collections::HashMap;

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig};
use crate::ui::art_deco_display::{ArtDecoFrameConfig, ArtDecoRenderer};

pub use rg_sens_types::display_configs::themed_configs::ArtDecoDisplayConfig;

// Use macro to generate displayer struct and basic implementations
crate::theme_displayer_base!(ArtDecoDisplayer, ArtDecoRenderer, ArtDecoRenderer);

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
