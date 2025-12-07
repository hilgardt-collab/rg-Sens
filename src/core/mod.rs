//! Core traits and types for rg-Sens

mod data_source;
mod displayer;
mod field_metadata;
mod panel;
mod registry;
mod update_manager;

pub use data_source::{BoxedDataSource, DataSource, SourceMetadata};
pub use displayer::{BoxedDisplayer, ConfigOption, ConfigSchema, Displayer};
pub use field_metadata::{FieldMetadata, FieldPurpose, FieldType};
pub use panel::{Panel, PanelGeometry, PanelBorderConfig};
pub use registry::{global_registry, Registry};
pub use update_manager::UpdateManager;
