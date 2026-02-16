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
use serde_json::Value;
use std::collections::HashMap;

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig};
use crate::ui::retro_terminal_display::{RetroTerminalFrameConfig, RetroTerminalRenderer};

pub use rg_sens_types::display_configs::themed_configs::RetroTerminalDisplayConfig;

// Use macro to generate displayer struct and basic implementations
crate::theme_displayer_base!(
    RetroTerminalDisplayer,
    RetroTerminalRenderer,
    RetroTerminalRenderer
);

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
