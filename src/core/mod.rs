//! Core traits and types for rg-Sens

mod data_source;
mod displayer;
mod panel;
mod registry;
mod update_manager;

pub use data_source::{BoxedDataSource, DataSource, SourceMetadata};
pub use displayer::{BoxedDisplayer, ConfigOption, ConfigSchema, Displayer};
pub use panel::{Panel, PanelGeometry};
pub use registry::{global_registry, Registry};
pub use update_manager::UpdateManager;
