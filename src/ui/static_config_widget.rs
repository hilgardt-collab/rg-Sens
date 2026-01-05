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
                config_clone.borrow_mut().text_overlay.text_config = text_widget_for_cb.get_config();
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
        self.text_enabled_check.set_active(new_config.text_overlay.enabled);
        self.text_widget.set_config(new_config.text_overlay.text_config);
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
