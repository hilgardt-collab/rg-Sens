//! Widget for configuring static display (background with optional text overlay)

use crate::core::FieldMetadata;
use crate::ui::lcars_display::StaticDisplayConfig;
use crate::ui::theme::ComboThemeConfig;
use crate::ui::BackgroundConfigWidget;
use crate::ui::TextLineConfigWidget;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, CheckButton, Frame, Label, Orientation, Widget};
use std::cell::RefCell;
use std::rc::Rc;

/// Widget for configuring static display with background and optional text overlay
pub struct StaticConfigWidget {
    widget: GtkBox,
    config: Rc<RefCell<StaticDisplayConfig>>,
    background_widget: Rc<BackgroundConfigWidget>,
    text_widget: Rc<TextLineConfigWidget>,
    text_enabled_check: CheckButton,
    #[allow(dead_code)] // Kept alive for GTK widget lifetime
    text_frame: Frame,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    theme: Rc<RefCell<ComboThemeConfig>>,
}

impl StaticConfigWidget {
    /// Create a new static config widget with available fields for text configuration
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 8);

        let config = Rc::new(RefCell::new(StaticDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let theme = Rc::new(RefCell::new(ComboThemeConfig::default()));

        // === Background Configuration Section ===
        let background_frame = Frame::new(Some("Background"));
        background_frame.set_margin_top(4);

        let background_widget = Rc::new(BackgroundConfigWidget::new());
        background_frame.set_child(Some(background_widget.widget()));
        widget.append(&background_frame);

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

        // === Text Overlay Section ===
        let text_frame = Frame::new(Some("Text Overlay"));
        text_frame.set_margin_top(8);

        let text_box = GtkBox::new(Orientation::Vertical, 6);
        text_box.set_margin_start(8);
        text_box.set_margin_end(8);
        text_box.set_margin_top(8);
        text_box.set_margin_bottom(8);

        // Enable text overlay checkbox
        let text_enabled_check = CheckButton::with_label("Enable Text Overlay");
        text_enabled_check.set_active(false);
        text_box.append(&text_enabled_check);

        // Info label
        let info_label = Label::new(Some(
            "When enabled, text lines will be rendered over the background.\nUse theme fonts and colors for consistent styling.",
        ));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        info_label.set_margin_top(4);
        text_box.append(&info_label);

        // Text lines configuration widget
        let text_widget = Rc::new(TextLineConfigWidget::new(available_fields));
        text_widget.widget().set_margin_top(8);
        text_box.append(text_widget.widget());

        // Initially hide text widget if not enabled
        text_widget.widget().set_visible(false);
        info_label.set_visible(false);

        text_frame.set_child(Some(&text_box));
        widget.append(&text_frame);

        // Connect text enabled checkbox
        {
            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            let text_widget_clone = text_widget.clone();
            let info_label_clone = info_label.clone();
            text_enabled_check.connect_toggled(move |check| {
                let is_enabled = check.is_active();
                text_widget_clone.widget().set_visible(is_enabled);
                info_label_clone.set_visible(is_enabled);
                config_clone.borrow_mut().text_overlay.enabled = is_enabled;
                if let Some(cb) = on_change_clone.borrow().as_ref() {
                    cb();
                }
            });
        }

        // Connect text widget on_change
        {
            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            let text_widget_for_cb = text_widget.clone();
            text_widget.set_on_change(move || {
                let text_config = text_widget_for_cb.get_config();
                config_clone.borrow_mut().text_overlay.text_config = text_config;
                if let Some(cb) = on_change_clone.borrow().as_ref() {
                    cb();
                }
            });
        }

        Self {
            widget,
            config,
            background_widget,
            text_widget,
            text_enabled_check,
            text_frame,
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
        self.text_widget.widget().set_visible(new_config.text_overlay.enabled);
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
