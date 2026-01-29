//! Trait for configuration widgets that can be lazily loaded
//!
//! This trait defines the common interface that all displayer configuration
//! widgets implement, allowing them to be used with the generic LazyConfigWidget.

use crate::core::FieldMetadata;
use crate::ui::theme::ComboThemeConfig;
use gtk4::Box as GtkBox;

/// Trait for configuration widgets that can be lazily loaded.
///
/// Implementing this trait allows a config widget to be wrapped in
/// `LazyConfigWidget<W>` for deferred initialization.
pub trait ConfigWidget: Sized {
    /// The configuration type this widget manages
    type Config: Clone + Default;

    /// Create a new config widget with the given available fields
    fn new(available_fields: Vec<FieldMetadata>) -> Self;

    /// Get a reference to the GTK widget container
    fn widget(&self) -> &GtkBox;

    /// Set the configuration
    fn set_config(&self, config: Self::Config);

    /// Get the current configuration
    fn get_config(&self) -> Self::Config;

    /// Set the callback invoked when configuration changes
    fn set_on_change<F: Fn() + 'static>(&self, callback: F);

    /// Set the theme configuration (optional, default does nothing)
    fn set_theme(&self, _theme: ComboThemeConfig) {}

    /// Cleanup resources and break reference cycles (optional, default does nothing)
    fn cleanup(&self) {}
}
