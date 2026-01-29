//! Widget for configuring static display (background with optional text overlay)

use crate::core::FieldMetadata;
use crate::ui::lcars_display::StaticDisplayConfig;
use crate::ui::theme::ComboThemeConfig;
use crate::ui::widget_builder::create_page_container;
use crate::ui::BackgroundConfigWidget;
use crate::ui::TextLineConfigWidget;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, CheckButton, Label, Notebook, Orientation, Widget};
use std::cell::RefCell;
use std::rc::Rc;

/// Widget for configuring static display with background and optional text overlay
pub struct StaticConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<StaticDisplayConfig>>,
    background_widget: Rc<BackgroundConfigWidget>,
    text_widget: Rc<TextLineConfigWidget>,
    text_enabled_check: CheckButton,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    theme: Rc<RefCell<ComboThemeConfig>>,
}

impl StaticConfigWidget {
    /// Create a new static config widget with available fields for text configuration
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 0);

        let config = Rc::new(RefCell::new(StaticDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let theme = Rc::new(RefCell::new(ComboThemeConfig::default()));

        // Create notebook for tabs
        let notebook = Notebook::new();
        notebook.set_vexpand(true);
        widget.append(&notebook);

        // === Tab 1: Background ===
        let background_page = GtkBox::new(Orientation::Vertical, 0);

        let background_widget = Rc::new(BackgroundConfigWidget::new());
        background_page.append(background_widget.widget());

        // Connect background widget on_change
        {
            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            let background_widget_for_cb = background_widget.clone();
            background_widget.set_on_change(move || {
                let bg_config = background_widget_for_cb.get_config();
                config_clone.borrow_mut().background = bg_config;
                if let Some(cb) = on_change_clone.borrow().as_ref() {
                    cb();
                }
            });
        }

        notebook.append_page(&background_page, Some(&Label::new(Some("Background"))));

        // === Tab 2: Text Overlay ===
        let text_page = create_page_container();

        let text_enabled_check = CheckButton::with_label("Show Text Overlay");
        text_enabled_check.set_active(false);
        text_page.append(&text_enabled_check);

        let text_widget = TextLineConfigWidget::new(available_fields);
        text_widget.widget().set_vexpand(true);
        text_page.append(text_widget.widget());
        let text_widget = Rc::new(text_widget);

        // Connect text config widget changes to trigger on_change callback
        {
            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            let text_widget_for_cb = text_widget.clone();
            text_widget.set_on_change(move || {
                config_clone.borrow_mut().text_overlay.text_config =
                    text_widget_for_cb.get_config();
                if let Some(cb) = on_change_clone.borrow().as_ref() {
                    cb();
                }
            });
        }

        // Connect text enabled checkbox
        {
            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            text_enabled_check.connect_toggled(move |check| {
                config_clone.borrow_mut().text_overlay.enabled = check.is_active();
                if let Some(cb) = on_change_clone.borrow().as_ref() {
                    cb();
                }
            });
        }

        notebook.append_page(&text_page, Some(&Label::new(Some("Text Overlay"))));

        Self {
            widget,
            config,
            background_widget,
            text_widget,
            text_enabled_check,
            on_change,
            theme,
        }
    }

    /// Get the widget
    pub fn widget(&self) -> &Widget {
        self.widget.upcast_ref()
    }

    /// Set the configuration
    pub fn set_config(&self, new_config: StaticDisplayConfig) {
        // Update internal state
        *self.config.borrow_mut() = new_config.clone();

        // Update background widget
        self.background_widget.set_config(new_config.background);

        // Update text overlay checkbox and widget
        self.text_enabled_check
            .set_active(new_config.text_overlay.enabled);
        self.text_widget
            .set_config(new_config.text_overlay.text_config);
    }

    /// Get the current configuration
    pub fn get_config(&self) -> StaticDisplayConfig {
        self.config.borrow().clone()
    }

    /// Set the on_change callback
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Set the theme config for resolving theme colors and fonts
    pub fn set_theme(&self, theme: ComboThemeConfig) {
        *self.theme.borrow_mut() = theme.clone();
        // Update child widgets with theme
        self.background_widget.set_theme_config(theme.clone());
        self.text_widget.set_theme(theme);
    }
}

/// Lazy wrapper for StaticConfigWidget to defer expensive widget creation
///
/// The actual StaticConfigWidget (with background editor, text overlay, etc.) is only created
/// when the widget becomes visible (mapped), saving memory when many
/// content items are created but only one display type is active.
pub struct LazyStaticConfigWidget {
    /// Container that holds either the placeholder or the actual widget
    container: GtkBox,
    /// The actual widget, created lazily on first map
    inner_widget: Rc<RefCell<Option<StaticConfigWidget>>>,
    /// Deferred config to apply when widget is created
    deferred_config: Rc<RefCell<StaticDisplayConfig>>,
    /// Deferred theme to apply when widget is created
    deferred_theme: Rc<RefCell<ComboThemeConfig>>,
    /// Available fields for the widget (used in init closure)
    #[allow(dead_code)]
    available_fields: Vec<FieldMetadata>,
    /// Callback to invoke on config changes
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    /// Signal handler ID for map callback, stored to disconnect during cleanup
    map_handler_id: Rc<RefCell<Option<gtk4::glib::SignalHandlerId>>>,
}

impl LazyStaticConfigWidget {
    /// Create a new lazy static config widget
    ///
    /// The actual StaticConfigWidget is NOT created here - it's created automatically
    /// when the widget becomes visible (mapped).
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);
        let inner_widget: Rc<RefCell<Option<StaticConfigWidget>>> = Rc::new(RefCell::new(None));
        let deferred_config = Rc::new(RefCell::new(StaticDisplayConfig::default()));
        let deferred_theme = Rc::new(RefCell::new(ComboThemeConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Create placeholder with loading indicator
        let placeholder = GtkBox::new(Orientation::Vertical, 8);
        placeholder.set_margin_top(12);
        placeholder.set_margin_bottom(12);
        placeholder.set_margin_start(12);
        placeholder.set_margin_end(12);

        let info_label = Label::new(Some("Loading static display configuration..."));
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

            Rc::new(move || {
                // Only create if not already created
                if inner_widget_clone.borrow().is_none() {
                    log::info!("LazyStaticConfigWidget: Creating actual StaticConfigWidget on map");

                    // Create the actual widget
                    let widget = StaticConfigWidget::new(available_fields_clone.clone());

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
        }
    }

    /// Get the widget container
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Set the static configuration
    pub fn set_config(&self, config: StaticDisplayConfig) {
        *self.deferred_config.borrow_mut() = config.clone();
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.set_config(config);
        }
    }

    /// Get the current static configuration
    pub fn get_config(&self) -> StaticDisplayConfig {
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.get_config()
        } else {
            self.deferred_config.borrow().clone()
        }
    }

    /// Set the theme for the static widget
    pub fn set_theme(&self, theme: ComboThemeConfig) {
        *self.deferred_theme.borrow_mut() = theme.clone();
        if let Some(ref widget) = *self.inner_widget.borrow() {
            widget.set_theme(theme);
        }
    }

    /// Set the on_change callback
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

    /// Cleanup method to break reference cycles and allow garbage collection.
    /// This clears the on_change callback which may hold Rc references to this widget.
    pub fn cleanup(&self) {
        log::debug!("LazyStaticConfigWidget::cleanup() - breaking reference cycles");
        // Disconnect the map signal handler to break the cycle
        if let Some(handler_id) = self.map_handler_id.borrow_mut().take() {
            self.container.disconnect(handler_id);
        }
        *self.on_change.borrow_mut() = None;
        *self.inner_widget.borrow_mut() = None;
    }
}
