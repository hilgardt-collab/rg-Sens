//! Configuration management

mod settings;
mod migration;

pub use settings::{AppConfig, GridConfig, PanelConfig, WindowConfig};
pub use migration::migrate_from_python;
