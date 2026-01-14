//! Configuration management

mod defaults;
mod settings;

pub use defaults::{DefaultsConfig, GeneralDefaults};
pub use settings::{AppConfig, GridConfig, PanelConfig, PanelConfigV2, WindowConfig};
