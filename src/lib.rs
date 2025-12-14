//! rg-Sens: A fast, customizable system monitoring dashboard for Linux
//!
//! This library provides the core functionality for rg-Sens, including:
//! - Data source traits and implementations for system metrics
//! - Display widgets for visualizing data
//! - Configuration management
//! - Plugin system architecture

// GTK code commonly uses complex callback types and let bindings for Cairo
#![allow(clippy::type_complexity)]
#![allow(clippy::let_unit_value)]
#![allow(clippy::single_match)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::if_same_then_else)]

pub mod audio;
pub mod core;
pub mod sources;
pub mod displayers;
pub mod ui;
pub mod config;
pub mod plugin;

// Re-export commonly used types
pub use core::{DataSource, Displayer, Panel, PanelData};
pub use config::{AppConfig, PanelConfig, PanelConfigV2};
