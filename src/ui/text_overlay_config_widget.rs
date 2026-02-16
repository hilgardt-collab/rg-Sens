//! Reusable text overlay configuration widget
//!
//! Provides a consistent UI for configuring text overlays across all displayers
//! (Arc, Bar, Speedometer, etc.). This ensures uniform behavior and prevents
//! bugs from inconsistent implementations.

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, CheckButton, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

use crate::core::FieldMetadata;
use crate::ui::config::{ConfigWidget, LazyConfigWidget};
use crate::ui::text_line_config_widget::TextLineConfigWidget;
use crate::ui::theme::ComboThemeConfig;

// Re-export TextOverlayConfig from rg-sens-types for backward compatibility
pub use rg_sens_types::text::TextOverlayConfig;

/// Reusable widget for configuring text overlays.
///
/// Encapsulates:
/// - Enable/disable checkbox
/// - TextLineConfigWidget for configuring text lines
/// - Theme support
/// - Consistent on_change handling
pub struct TextOverlayConfigWidget {
    container: GtkBox,
    enable_check: CheckButton,
    text_widget: Rc<TextLineConfigWidget>,
    config: Rc<RefCell<TextOverlayConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    theme: Rc<RefCell<ComboThemeConfig>>,
}

impl TextOverlayConfigWidget {
    /// Create a new text overlay config widget with the given available fields
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 8);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);

        let config = Rc::new(RefCell::new(TextOverlayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let theme = Rc::new(RefCell::new(ComboThemeConfig::default()));

        // Enable checkbox
        let enable_check = CheckButton::with_label("Enable Text Overlay");
        enable_check.set_active(config.borrow().enabled);
        container.append(&enable_check);

        // Text line config widget
        let text_widget = TextLineConfigWidget::new(available_fields);
        text_widget.set_config(config.borrow().text_config.clone());
        let text_widget = Rc::new(text_widget);

        container.append(text_widget.widget());

        // Connect enable checkbox
        {
            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            enable_check.connect_toggled(move |check| {
                config_clone.borrow_mut().enabled = check.is_active();
                if let Some(ref callback) = *on_change_clone.borrow() {
                    callback();
                }
            });
        }

        // Connect text widget on_change
        {
            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            let text_widget_for_callback = text_widget.clone();
            text_widget.set_on_change(move || {
                // Sync text config from widget to internal config
                config_clone.borrow_mut().text_config = text_widget_for_callback.get_config();
                // Notify parent
                if let Some(ref callback) = *on_change_clone.borrow() {
                    callback();
                }
            });
        }

        Self {
            container,
            enable_check,
            text_widget,
            config,
            on_change,
            theme,
        }
    }

    /// Get the GTK widget
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Set the configuration
    pub fn set_config(&self, new_config: TextOverlayConfig) {
        *self.config.borrow_mut() = new_config.clone();

        // Update UI
        self.enable_check.set_active(new_config.enabled);
        self.text_widget.set_config(new_config.text_config);
    }

    /// Get the current configuration
    pub fn get_config(&self) -> TextOverlayConfig {
        TextOverlayConfig {
            enabled: self.enable_check.is_active(),
            text_config: self.text_widget.get_config(),
        }
    }

    /// Set the on_change callback
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Set the theme for color/font resolution
    pub fn set_theme(&self, new_theme: ComboThemeConfig) {
        *self.theme.borrow_mut() = new_theme.clone();
        self.text_widget.set_theme(new_theme);
    }

    /// Get the theme
    pub fn get_theme(&self) -> ComboThemeConfig {
        self.theme.borrow().clone()
    }

    /// Cleanup method to break reference cycles
    pub fn cleanup(&self) {
        *self.on_change.borrow_mut() = None;
    }
}

impl Default for TextOverlayConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl ConfigWidget for TextOverlayConfigWidget {
    type Config = TextOverlayConfig;

    fn new(available_fields: Vec<FieldMetadata>) -> Self {
        TextOverlayConfigWidget::new(available_fields)
    }

    fn widget(&self) -> &GtkBox {
        &self.container
    }

    fn set_config(&self, config: Self::Config) {
        TextOverlayConfigWidget::set_config(self, config);
    }

    fn get_config(&self) -> Self::Config {
        TextOverlayConfigWidget::get_config(self)
    }

    fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        TextOverlayConfigWidget::set_on_change(self, callback);
    }

    fn set_theme(&self, theme: ComboThemeConfig) {
        TextOverlayConfigWidget::set_theme(self, theme);
    }

    fn cleanup(&self) {
        TextOverlayConfigWidget::cleanup(self);
    }
}

// =============================================================================
// LazyTextOverlayConfigWidget - Type alias using generic LazyConfigWidget
// =============================================================================

/// Lazy-loading wrapper for TextOverlayConfigWidget.
pub type LazyTextOverlayConfigWidget = LazyConfigWidget<TextOverlayConfigWidget>;
