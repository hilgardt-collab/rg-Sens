//! Core traits and types for rg-Sens

mod data_source;
mod displayer;
mod panel;
mod registry;
mod update_manager;

pub use data_source::{DataSource, SourceMetadata};
pub use displayer::{Displayer, ConfigSchema};
pub use panel::Panel;
pub use registry::Registry;
pub use update_manager::UpdateManager;
