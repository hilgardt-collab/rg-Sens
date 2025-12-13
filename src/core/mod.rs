//! Core traits and types for rg-Sens

mod data_source;
mod displayer;
mod field_metadata;
mod panel;
mod panel_data;
mod registry;
mod update_manager;

pub use data_source::{BoxedDataSource, DataSource, SourceMetadata};
pub use displayer::{BoxedDisplayer, ConfigOption, ConfigSchema, Displayer};
pub use field_metadata::{FieldMetadata, FieldPurpose, FieldType};
pub use panel::{Panel, PanelGeometry, PanelBorderConfig};
pub use panel_data::{PanelData, PanelAppearance, SourceConfig, DisplayerConfig};
pub use registry::{global_registry, Registry, SourceInfo, DisplayerInfo};
pub use update_manager::{UpdateManager, init_global_update_manager, global_update_manager};
