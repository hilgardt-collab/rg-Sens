//! Configuration management

mod settings;
mod migration;

pub use settings::{AppConfig, GridConfig, PanelConfig, PanelConfigV2, WindowConfig};
pub use migration::migrate_from_python;
