//! Core traits and types for rg-Sens
//!
//! Re-exports core traits from the rg-sens-core crate,
//! plus GTK-heavy modules that stay in the main crate.

// Re-export everything from the core crate
pub use rg_sens_core::*;

// Main-crate-only modules (GTK/runtime-heavy)
mod animation_manager;
mod panel;
mod panel_data;
mod shared_source_manager;
mod timer_manager;
mod update_manager;

pub use animation_manager::{
    animation_entry_count, check_animation_stall, init_global_animation_manager,
    register_animation, shutdown_animation_manager,
};
pub use panel::{Panel, PanelBorderConfig, PanelGeometry};
pub use panel_data::{DisplayerConfig, PanelAppearance, PanelData, SourceConfig};
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
