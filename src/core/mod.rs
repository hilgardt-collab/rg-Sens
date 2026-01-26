//! Core traits and types for rg-Sens

mod animation_manager;
pub mod constants;
mod data_source;
mod displayer;
mod field_metadata;
mod panel;
mod panel_data;
mod registry;
mod shared_source_manager;
mod timer_manager;
mod update_manager;

pub use animation_manager::{animation_entry_count, init_global_animation_manager, register_animation};
pub use constants::{
    ANIMATION_FRAME_INTERVAL, ANIMATION_FRAME_MS, ANIMATION_SNAP_THRESHOLD, STATIC_POLL_INTERVAL,
};
pub use data_source::{BoxedDataSource, DataSource, SourceMetadata};
pub use displayer::{BoxedDisplayer, ConfigOption, ConfigSchema, Displayer, PanelTransform};
pub use field_metadata::{FieldMetadata, FieldPurpose, FieldType};
pub use panel::{Panel, PanelBorderConfig, PanelGeometry};
pub use panel_data::{DisplayerConfig, PanelAppearance, PanelData, SourceConfig};
pub use registry::{global_registry, DisplayerInfo, Registry, SourceInfo};
pub use shared_source_manager::{
    global_shared_source_manager, init_global_shared_source_manager, SharedSource,
    SharedSourceManager,
};
pub use timer_manager::{
    global_timer_manager, play_preview_sound, shutdown_audio_thread, stop_all_sounds, AlarmConfig,
    TimerAlarmManager, TimerConfig, TimerDisplayConfig, TimerMode, TimerState,
};
pub use update_manager::{
    check_update_stall, global_update_manager, init_global_update_manager, UpdateManager,
};
