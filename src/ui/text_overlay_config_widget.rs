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
use crate::displayers::TextDisplayerConfig;
use crate::ui::text_line_config_widget::TextLineConfigWidget;
use crate::ui::theme::ComboThemeConfig;

/// Configuration for text overlay (shared structure used by multiple displayers)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TextOverlayConfig {
    pub enabled: bool,
    #[serde(default)]
    pub text_config: TextDisplayerConfig,
}

impl Default for TextOverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            text_config: TextDisplayerConfig::default(),
        }
    }
}

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
}

impl Default for TextOverlayConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

// =============================================================================
// LazyTextOverlayConfigWidget - Delays creation of TextOverlayConfigWidget until needed
// =============================================================================

/// A lazy-loading wrapper for TextOverlayConfigWidget that defers expensive widget creation
/// until the user actually clicks to expand/configure the text overlay.
///
/// This significantly improves dialog open time for combo panels with many slots,
/// as TextOverlayConfigWidget creation is deferred until needed.
pub struct LazyTextOverlayConfigWidget {
    /// Container that holds either the placeholder or the actual widget
    container: GtkBox,
    /// The actual widget, created lazily on first expand
    inner_widget: Rc<RefCell<Option<TextOverlayConfigWidget>>>,
    /// Deferred config to apply when widget is created
    deferred_config: Rc<RefCell<TextOverlayConfig>>,
    /// Deferred theme to apply when widget is created
    deferred_theme: Rc<RefCell<ComboThemeConfig>>,
    /// Available fields for the widget
    available_fields: Vec<FieldMetadata>,
    /// Callback to invoke on config changes
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
}

impl LazyTextOverlayConfigWidget {
    /// Create a new lazy text overlay config widget
    ///
    /// The actual TextOverlayConfigWidget is NOT created here - it's created automatically
    /// when the widget becomes visible (mapped), or when explicitly initialized.
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);
        let inner_widget: Rc<RefCell<Option<TextOverlayConfigWidget>>> =
            Rc::new(RefCell::new(None));
        let deferred_config = Rc::new(RefCell::new(TextOverlayConfig::default()));
        let deferred_theme = Rc::new(RefCell::new(ComboThemeConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Set up lazy initialization when widget becomes visible
        let inner_widget_clone = inner_widget.clone();
        let container_clone = container.clone();
        let deferred_config_clone = deferred_config.clone();
        let deferred_theme_clone = deferred_theme.clone();
        let on_change_clone = on_change.clone();
        let fields_clone = available_fields.clone();

        container.connect_map(move |_| {
            let mut inner = inner_widget_clone.borrow_mut();
            if inner.is_none() {
                // Create the actual widget now
                let widget = TextOverlayConfigWidget::new(fields_clone.clone());
                widget.set_theme(deferred_theme_clone.borrow().clone());
                widget.set_config(deferred_config_clone.borrow().clone());

                // Connect on_change callback
                let on_change_inner = on_change_clone.clone();
                widget.set_on_change(move || {
                    if let Some(ref callback) = *on_change_inner.borrow() {
                        callback();
                    }
                });

                container_clone.append(widget.widget());
                *inner = Some(widget);
            }
        });

        Self {
            container,
            inner_widget,
            deferred_config,
            deferred_theme,
            available_fields,
            on_change,
        }
    }

    /// Get the GTK widget
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Set the configuration (deferred if widget not yet created)
    pub fn set_config(&self, config: TextOverlayConfig) {
        *self.deferred_config.borrow_mut() = config.clone();
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.set_config(config);
        }
    }

    /// Get the current configuration
    pub fn get_config(&self) -> TextOverlayConfig {
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.get_config()
        } else {
            self.deferred_config.borrow().clone()
        }
    }

    /// Set the on_change callback
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
        // If widget already exists, reconnect it
        if let Some(ref widget) = *self.inner_widget.borrow() {
            let on_change_clone = self.on_change.clone();
            widget.set_on_change(move || {
                if let Some(ref cb) = *on_change_clone.borrow() {
                    cb();
                }
            });
        }
    }

    /// Set the theme (deferred if widget not yet created)
    pub fn set_theme(&self, theme: ComboThemeConfig) {
        *self.deferred_theme.borrow_mut() = theme.clone();
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.set_theme(theme);
        }
    }

    /// Force initialization of the inner widget (useful for testing or pre-loading)
    #[allow(dead_code)]
    pub fn ensure_initialized(&self) {
        let mut inner = self.inner_widget.borrow_mut();
        if inner.is_none() {
            let widget = TextOverlayConfigWidget::new(self.available_fields.clone());
            widget.set_theme(self.deferred_theme.borrow().clone());
            widget.set_config(self.deferred_config.borrow().clone());

            let on_change_clone = self.on_change.clone();
            widget.set_on_change(move || {
                if let Some(ref callback) = *on_change_clone.borrow() {
                    callback();
                }
            });

            self.container.append(widget.widget());
            *inner = Some(widget);
        }
    }
}
