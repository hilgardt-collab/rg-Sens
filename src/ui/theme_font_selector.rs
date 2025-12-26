//! Theme font selector widget
//!
//! Provides a row of theme font toggle buttons (T1-T2) plus a custom font picker.
//! Used throughout combo panel config dialogs to select either a theme font or custom font.

use crate::ui::shared_font_dialog::shared_font_dialog;
use crate::ui::theme::{ComboThemeConfig, FontSource};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Orientation, SpinButton, ToggleButton};
use std::cell::RefCell;
use std::rc::Rc;

/// A widget for selecting either a theme font (T1-T2) or a custom font.
///
/// Layout: [T1][T2] [Font Button] Size: [Spin]
///
/// When a theme button is active, the custom font controls are dimmed.
/// When "Custom" mode is active (no theme button selected), the controls are enabled.
pub struct ThemeFontSelector {
    container: GtkBox,
    theme_buttons: [ToggleButton; 2],
    font_button: Button,
    size_spin: SpinButton,
    source: Rc<RefCell<FontSource>>,
    custom_family: Rc<RefCell<String>>,
    custom_size: Rc<RefCell<f64>>,
    theme_config: Rc<RefCell<Option<ComboThemeConfig>>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn(FontSource)>>>>,
}

impl ThemeFontSelector {
    /// Create a new ThemeFontSelector with the given initial source.
    pub fn new(initial_source: FontSource) -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 4);
        let source = Rc::new(RefCell::new(initial_source.clone()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn(FontSource)>>>> = Rc::new(RefCell::new(None));
        let theme_config: Rc<RefCell<Option<ComboThemeConfig>>> = Rc::new(RefCell::new(None));

        // Extract custom font from source, or default
        let (initial_family, initial_size) = match &initial_source {
            FontSource::Custom { family, size } => (family.clone(), *size),
            FontSource::Theme { .. } => ("sans-serif".to_string(), 12.0),
        };
        let custom_family = Rc::new(RefCell::new(initial_family.clone()));
        let custom_size = Rc::new(RefCell::new(initial_size));

        // Create theme toggle buttons
        let theme_buttons: [ToggleButton; 2] = [
            ToggleButton::with_label("T1"),
            ToggleButton::with_label("T2"),
        ];

        // Set tooltips
        theme_buttons[0].set_tooltip_text(Some("Theme Font 1 (Headers)"));
        theme_buttons[1].set_tooltip_text(Some("Theme Font 2 (Content)"));

        // Group the toggle buttons so only one can be active at a time
        // (or none, for custom mode)
        theme_buttons[1].set_group(Some(&theme_buttons[0]));

        // Set initial active state based on source
        if let FontSource::Theme { index } = &initial_source {
            let idx = (*index as usize).saturating_sub(1).min(1);
            theme_buttons[idx].set_active(true);
        }

        // Add theme buttons to container
        for btn in &theme_buttons {
            container.append(btn);
        }

        // Add separator
        let separator = gtk4::Separator::new(Orientation::Vertical);
        separator.set_margin_start(4);
        separator.set_margin_end(4);
        container.append(&separator);

        // Create font button
        let font_button = Button::with_label(&initial_family);
        font_button.set_hexpand(true);
        font_button.set_tooltip_text(Some("Custom font family (click to change)"));
        container.append(&font_button);

        // Create size label and spin
        let size_label = gtk4::Label::new(Some("Size:"));
        size_label.set_margin_start(8);
        container.append(&size_label);

        let size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        size_spin.set_value(initial_size);
        size_spin.set_tooltip_text(Some("Custom font size"));
        container.append(&size_spin);

        // Dim controls if theme is selected
        if initial_source.is_theme() {
            font_button.set_sensitive(false);
            size_spin.set_sensitive(false);
        }

        // Connect theme button toggled handlers
        for (i, btn) in theme_buttons.iter().enumerate() {
            let source_clone = source.clone();
            let on_change_clone = on_change.clone();
            let font_button_clone = font_button.clone();
            let size_spin_clone = size_spin.clone();
            let theme_config_clone = theme_config.clone();

            btn.connect_toggled(move |toggle_btn| {
                if toggle_btn.is_active() {
                    let new_source = FontSource::Theme { index: (i + 1) as u8 };
                    *source_clone.borrow_mut() = new_source.clone();
                    font_button_clone.set_sensitive(false);
                    size_spin_clone.set_sensitive(false);

                    // Update button label to show theme font
                    if let Some(ref cfg) = *theme_config_clone.borrow() {
                        let (family, _size) = cfg.get_font((i + 1) as u8);
                        font_button_clone.set_label(&family);
                    }

                    if let Some(ref callback) = *on_change_clone.borrow() {
                        callback(new_source);
                    }
                }
            });
        }

        // Connect font button click handler (for custom font)
        let custom_family_clone = custom_family.clone();
        let custom_size_clone = custom_size.clone();
        let source_for_click = source.clone();
        let on_change_for_click = on_change.clone();
        let theme_buttons_clone: Vec<ToggleButton> = theme_buttons.iter().cloned().collect();
        let font_button_for_click = font_button.clone();
        let size_spin_for_click = size_spin.clone();

        font_button.connect_clicked(move |btn| {
            let current_family = custom_family_clone.borrow().clone();
            let window = btn
                .root()
                .and_then(|root| root.downcast::<gtk4::Window>().ok());

            if let Some(window) = window {
                let font_desc = gtk4::pango::FontDescription::from_string(&current_family);
                let custom_family_clone2 = custom_family_clone.clone();
                let custom_size_clone2 = custom_size_clone.clone();
                let source_clone2 = source_for_click.clone();
                let on_change_clone2 = on_change_for_click.clone();
                let theme_buttons_clone2 = theme_buttons_clone.clone();
                let font_button_clone2 = font_button_for_click.clone();
                let size_spin_clone2 = size_spin_for_click.clone();

                shared_font_dialog().choose_font(
                    Some(&window),
                    Some(&font_desc),
                    gtk4::gio::Cancellable::NONE,
                    move |result| {
                        if let Ok(new_font_desc) = result {
                            let family = new_font_desc
                                .family()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "sans-serif".to_string());

                            *custom_family_clone2.borrow_mut() = family.clone();
                            let size = *custom_size_clone2.borrow();
                            let new_source = FontSource::Custom { family: family.clone(), size };
                            *source_clone2.borrow_mut() = new_source.clone();

                            // Deselect all theme buttons
                            for btn in &theme_buttons_clone2 {
                                btn.set_active(false);
                            }
                            font_button_clone2.set_label(&family);
                            font_button_clone2.set_sensitive(true);
                            size_spin_clone2.set_sensitive(true);

                            if let Some(ref callback) = *on_change_clone2.borrow() {
                                callback(new_source);
                            }
                        }
                    },
                );
            }
        });

        // Connect size spin handler
        let custom_family_for_spin = custom_family.clone();
        let custom_size_for_spin = custom_size.clone();
        let source_for_spin = source.clone();
        let on_change_for_spin = on_change.clone();
        let theme_buttons_for_spin: Vec<ToggleButton> = theme_buttons.iter().cloned().collect();
        let font_button_for_spin = font_button.clone();
        let size_spin_for_spin_handler = size_spin.clone();

        size_spin.connect_value_changed(move |spin| {
            let new_size = spin.value();
            *custom_size_for_spin.borrow_mut() = new_size;

            // If we're in theme mode, switch to custom
            let current_source = source_for_spin.borrow().clone();
            if current_source.is_theme() {
                // Deselect all theme buttons
                for btn in &theme_buttons_for_spin {
                    btn.set_active(false);
                }
                font_button_for_spin.set_sensitive(true);
                size_spin_for_spin_handler.set_sensitive(true);
            }

            let family = custom_family_for_spin.borrow().clone();
            let new_source = FontSource::Custom { family, size: new_size };
            *source_for_spin.borrow_mut() = new_source.clone();

            if let Some(ref callback) = *on_change_for_spin.borrow() {
                callback(new_source);
            }
        });

        Self {
            container,
            theme_buttons,
            font_button,
            size_spin,
            source,
            custom_family,
            custom_size,
            theme_config,
            on_change,
        }
    }

    /// Get the container widget (for adding to layouts).
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Get the current font source.
    pub fn source(&self) -> FontSource {
        self.source.borrow().clone()
    }

    /// Set the font source (updates the UI).
    pub fn set_source(&self, source: FontSource) {
        *self.source.borrow_mut() = source.clone();

        match &source {
            FontSource::Theme { index } => {
                let idx = (*index as usize).saturating_sub(1).min(1);
                self.theme_buttons[idx].set_active(true);
                self.font_button.set_sensitive(false);
                self.size_spin.set_sensitive(false);

                // Update button label to show theme font
                if let Some(ref cfg) = *self.theme_config.borrow() {
                    let (family, _size) = cfg.get_font(*index);
                    self.font_button.set_label(&family);
                }
            }
            FontSource::Custom { family, size } => {
                // Deselect all theme buttons
                for btn in &self.theme_buttons {
                    btn.set_active(false);
                }
                *self.custom_family.borrow_mut() = family.clone();
                *self.custom_size.borrow_mut() = *size;
                self.font_button.set_label(family);
                self.size_spin.set_value(*size);
                self.font_button.set_sensitive(true);
                self.size_spin.set_sensitive(true);
            }
        }
    }

    /// Set the theme config (used to resolve theme fonts for display).
    pub fn set_theme_config(&self, config: ComboThemeConfig) {
        *self.theme_config.borrow_mut() = Some(config.clone());

        // Update button label if currently using theme
        let source = self.source.borrow().clone();
        if let FontSource::Theme { index } = source {
            let (family, _size) = config.get_font(index);
            self.font_button.set_label(&family);
        }
    }

    /// Set a callback to be called when the source changes.
    pub fn set_on_change<F: Fn(FontSource) + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Resolve the current source to an actual font (family, size).
    pub fn resolve_font(&self) -> (String, f64) {
        let source = self.source.borrow().clone();
        match &source {
            FontSource::Custom { family, size } => (family.clone(), *size),
            FontSource::Theme { index } => {
                if let Some(ref cfg) = *self.theme_config.borrow() {
                    cfg.get_font(*index)
                } else {
                    (self.custom_family.borrow().clone(), *self.custom_size.borrow())
                }
            }
        }
    }
}
