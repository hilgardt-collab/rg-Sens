//! Generic lazy loading wrapper for configuration widgets
//!
//! This replaces the 8 separate LazyXxxConfigWidget implementations with a single
//! generic implementation, eliminating ~1600 lines of duplicated code.

use super::ConfigWidget;
use crate::core::FieldMetadata;
use crate::ui::theme::ComboThemeConfig;
use crate::ui::widget_builder::DEFAULT_MARGIN;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

/// Generic lazy loading wrapper for configuration widgets.
///
/// This wrapper defers the creation of the actual configuration widget until
/// the container is first mapped (becomes visible). This significantly improves
/// dialog opening performance when there are many tabs.
///
/// # Type Parameters
///
/// * `W` - The concrete config widget type that implements `ConfigWidget`
///
/// # Example
///
/// ```ignore
/// // Instead of implementing LazyBarConfigWidget manually (~200 lines),
/// // just use the type alias:
/// pub type LazyBarConfigWidget = LazyConfigWidget<BarConfigWidget>;
/// ```
pub struct LazyConfigWidget<W: ConfigWidget> {
    /// Container that holds either the placeholder or the actual widget
    container: GtkBox,
    /// The actual widget, created lazily on first map
    inner_widget: Rc<RefCell<Option<W>>>,
    /// Deferred config to apply when widget is created
    deferred_config: Rc<RefCell<W::Config>>,
    /// Deferred theme to apply when widget is created
    deferred_theme: Rc<RefCell<ComboThemeConfig>>,
    /// Available fields for the widget
    available_fields: Vec<FieldMetadata>,
    /// Callback to invoke on config changes
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    /// Signal handler ID for map callback, stored to disconnect during cleanup
    map_handler_id: Rc<RefCell<Option<gtk4::glib::SignalHandlerId>>>,
    /// Phantom data to satisfy the type system
    _phantom: PhantomData<W>,
}

impl<W: ConfigWidget + 'static> LazyConfigWidget<W> {
    /// Create a new lazy config widget.
    ///
    /// The actual widget is NOT created here - it's created automatically
    /// when the container becomes visible (mapped), or when explicitly initialized.
    ///
    /// # Arguments
    ///
    /// * `available_fields` - The field metadata available for configuration
    /// * `widget_name` - Human-readable name shown in the loading placeholder
    pub fn new(available_fields: Vec<FieldMetadata>, widget_name: &str) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);
        let inner_widget: Rc<RefCell<Option<W>>> = Rc::new(RefCell::new(None));
        let deferred_config = Rc::new(RefCell::new(W::Config::default()));
        let deferred_theme = Rc::new(RefCell::new(ComboThemeConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Create placeholder with loading indicator
        let placeholder = GtkBox::new(Orientation::Vertical, 8);
        placeholder.set_margin_top(DEFAULT_MARGIN);
        placeholder.set_margin_bottom(DEFAULT_MARGIN);
        placeholder.set_margin_start(DEFAULT_MARGIN);
        placeholder.set_margin_end(DEFAULT_MARGIN);

        let info_label = Label::new(Some(&format!("Loading {} configuration...", widget_name)));
        info_label.add_css_class("dim-label");
        placeholder.append(&info_label);
        container.append(&placeholder);

        // Create a shared initialization closure
        let init_widget = {
            let container_clone = container.clone();
            let inner_widget_clone = inner_widget.clone();
            let deferred_config_clone = deferred_config.clone();
            let deferred_theme_clone = deferred_theme.clone();
            let available_fields_clone = available_fields.clone();
            let on_change_clone = on_change.clone();
            let widget_name = widget_name.to_string();

            Rc::new(move || {
                // Only create if not already created
                if inner_widget_clone.borrow().is_none() {
                    log::info!(
                        "LazyConfigWidget<{}>: Creating actual widget on map",
                        widget_name
                    );

                    // Create the actual widget
                    let widget = W::new(available_fields_clone.clone());

                    // Apply deferred theme first (before config, as config may trigger UI updates)
                    widget.set_theme(deferred_theme_clone.borrow().clone());

                    // Apply deferred config
                    widget.set_config(deferred_config_clone.borrow().clone());

                    // Connect on_change callback
                    let on_change_inner = on_change_clone.clone();
                    widget.set_on_change(move || {
                        if let Some(ref callback) = *on_change_inner.borrow() {
                            callback();
                        }
                    });

                    // Remove placeholder and add actual widget
                    while let Some(child) = container_clone.first_child() {
                        container_clone.remove(&child);
                    }
                    container_clone.append(widget.widget());

                    // Store the widget
                    *inner_widget_clone.borrow_mut() = Some(widget);
                }
            })
        };

        // Auto-initialize when the widget becomes visible (mapped)
        // Store the handler ID so we can disconnect during cleanup to break the cycle
        let map_handler_id: Rc<RefCell<Option<gtk4::glib::SignalHandlerId>>> =
            Rc::new(RefCell::new(None));
        {
            let init_widget_clone = init_widget.clone();
            let handler_id = container.connect_map(move |_| {
                init_widget_clone();
            });
            *map_handler_id.borrow_mut() = Some(handler_id);
        }

        Self {
            container,
            inner_widget,
            deferred_config,
            deferred_theme,
            available_fields,
            on_change,
            map_handler_id,
            _phantom: PhantomData,
        }
    }

    /// Get the widget container
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Set the configuration.
    ///
    /// If the inner widget hasn't been created yet, this stores the config
    /// to be applied when it is created. Otherwise, it's applied immediately.
    pub fn set_config(&self, config: W::Config) {
        *self.deferred_config.borrow_mut() = config.clone();
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.set_config(config);
        }
    }

    /// Get the current configuration.
    ///
    /// Returns the deferred config if the inner widget hasn't been created yet.
    pub fn get_config(&self) -> W::Config {
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.get_config()
        } else {
            self.deferred_config.borrow().clone()
        }
    }

    /// Set the theme for the widget.
    pub fn set_theme(&self, theme: ComboThemeConfig) {
        *self.deferred_theme.borrow_mut() = theme.clone();
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.set_theme(theme);
        }
    }

    /// Set the on_change callback.
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
        // If widget already exists, connect it
        if let Some(ref widget) = *self.inner_widget.borrow() {
            let on_change_inner = self.on_change.clone();
            widget.set_on_change(move || {
                if let Some(ref cb) = *on_change_inner.borrow() {
                    cb();
                }
            });
        }
    }

    /// Check if the actual widget has been created.
    pub fn is_initialized(&self) -> bool {
        self.inner_widget.borrow().is_some()
    }

    /// Force initialization of the inner widget (for cases where it must exist).
    pub fn ensure_initialized(&self) {
        if self.inner_widget.borrow().is_none() {
            log::info!("LazyConfigWidget: Force-initializing widget");

            let widget = W::new(self.available_fields.clone());
            widget.set_theme(self.deferred_theme.borrow().clone());
            widget.set_config(self.deferred_config.borrow().clone());

            // Connect on_change
            let on_change_inner = self.on_change.clone();
            widget.set_on_change(move || {
                if let Some(ref callback) = *on_change_inner.borrow() {
                    callback();
                }
            });

            // Remove placeholder and add actual widget
            while let Some(child) = self.container.first_child() {
                self.container.remove(&child);
            }
            self.container.append(widget.widget());

            *self.inner_widget.borrow_mut() = Some(widget);
        }
    }

    /// Cleanup method to break reference cycles and allow garbage collection.
    ///
    /// This clears the on_change callback which may hold Rc references to this widget,
    /// and disconnects the map signal handler to break the container->closure->container cycle.
    pub fn cleanup(&self) {
        log::debug!("LazyConfigWidget::cleanup() - breaking reference cycles");
        // Disconnect the map signal handler to break the cycle
        if let Some(handler_id) = self.map_handler_id.borrow_mut().take() {
            self.container.disconnect(handler_id);
        }
        // Cleanup inner widget if it exists
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.cleanup();
        }
        *self.on_change.borrow_mut() = None;
        *self.inner_widget.borrow_mut() = None;
    }
}
