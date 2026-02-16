//! rg-sens-core: Core traits and registry for rg-Sens system monitor.
//!
//! This crate contains the fundamental traits (DataSource, Displayer),
//! the global Registry, and shared constants.

pub mod constants;
mod data_source;
mod displayer;
mod registry;

pub use constants::{
    ANIMATION_FRAME_INTERVAL, ANIMATION_FRAME_MS, ANIMATION_SNAP_THRESHOLD, BYTES_PER_GB,
    BYTES_PER_KB, BYTES_PER_MB, BYTES_PER_TB, STATIC_POLL_INTERVAL, TRANSFORM_THRESHOLD,
};
pub use data_source::{BoxedDataSource, DataSource, SourceMetadata};
pub use displayer::{BoxedDisplayer, ConfigOption, ConfigSchema, Displayer, PanelTransform};
pub use registry::{
    global_registry, DisplayerFactory, DisplayerInfo, Registry, SourceFactory, SourceInfo,
};

// Re-export types used in trait signatures for convenience
pub use rg_sens_types::panel::{DisplayerConfig, SourceConfig};
pub use rg_sens_types::{FieldMetadata, FieldPurpose, FieldType};
