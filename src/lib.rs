//! rg-Sens: A fast, customizable system monitoring dashboard for Linux
//!
//! This library provides the core functionality for rg-Sens, including:
//! - Data source traits and implementations for system metrics
//! - Display widgets for visualizing data
//! - Configuration management
//! - Plugin system architecture

pub mod core;
pub mod sources;
pub mod displayers;
pub mod ui;
pub mod config;
pub mod plugin;

// Re-export commonly used types
pub use core::{DataSource, Displayer, Panel};
pub use config::{AppConfig, PanelConfig};
