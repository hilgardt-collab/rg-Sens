//! Configuration widget infrastructure
//!
//! This module provides generic lazy loading for configuration widgets,
//! reducing code duplication across the many config widget implementations.

mod config_widget_trait;
mod lazy_config_widget;

pub use config_widget_trait::ConfigWidget;
pub use lazy_config_widget::LazyConfigWidget;
