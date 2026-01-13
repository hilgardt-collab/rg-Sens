//! Configuration management

mod settings;
mod defaults;

pub use settings::{AppConfig, GridConfig, PanelConfig, PanelConfigV2, WindowConfig};
pub use defaults::{DefaultsConfig, GeneralDefaults};
