//! Configuration management

mod settings;
mod migration;

pub use settings::{AppConfig, PanelConfig};
pub use migration::migrate_from_python;
