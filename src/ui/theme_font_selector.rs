//! Theme font selector widget
//!
//! Provides a row of theme font toggle buttons (T1-T2) plus a custom font picker.
//! T1/T2 controls font family from theme, size is always independent.

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
/// T1/T2 buttons select font family from theme.
/// Size spinner is always independent.
pub struct ThemeFontSelector {
    container: GtkBox,
    theme_buttons: [ToggleButton; 2],
    font_button: Button,
    size_spin: SpinButton,
    /// Which theme font index is active (1, 2) or None for custom
    theme_index: Rc<RefCell<Option<u8>>>,
    custom_family: Rc<RefCell<String>>,
    theme_config: Rc<RefCell<Option<ComboThemeConfig>>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn(FontSource)>>>>,
}

impl ThemeFontSelector {
    /// Create a new ThemeFontSelector with the given initial source.
    pub fn new(initial_source: FontSource) -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 4);
        let on_change: Rc<RefCell<Option<Box<dyn Fn(FontSource)>>>> = Rc::new(RefCell::new(None));
        let theme_config: Rc<RefCell<Option<ComboThemeConfig>>> = Rc::new(RefCell::new(None));

        // Extract initial values
        let (initial_family, initial_size, initial_theme_index) = match &initial_source {
            FontSource::Custom { family, size } => (family.clone(), *size, None),
            FontSource::Theme { index, size } => ("sans-serif".to_string(), *size, Some(*index)),
        };
        let custom_family = Rc::new(RefCell::new(initial_family.clone()));
        let theme_index = Rc::new(RefCell::new(initial_theme_index));

        // Create theme toggle buttons
        let theme_buttons: [ToggleButton; 2] = [
            ToggleButton::with_label("T1"),
            ToggleButton::with_label("T2"),
        ];

        theme_buttons[0].set_tooltip_text(Some("Use Theme Font 1"));
        theme_buttons[1].set_tooltip_text(Some("Use Theme Font 2"));
        theme_buttons[1].set_group(Some(&theme_buttons[0]));

        // Set initial active state
        if let Some(idx) = initial_theme_index {
            let btn_idx = (idx as usize).saturating_sub(1).min(1);
            theme_buttons[btn_idx].set_active(true);
        }

        for btn in &theme_buttons {
            container.append(btn);
        }

        // Separator
        let separator = gtk4::Separator::new(Orientation::Vertical);
        separator.set_margin_start(4);
        separator.set_margin_end(4);
        container.append(&separator);

        // Font button (shows current family)
        let font_button = Button::with_label(&initial_family);
        font_button.set_hexpand(true);
        font_button.set_tooltip_text(Some("Click to choose custom font"));
        container.append(&font_button);

        // Size spinner (always independent)
        let size_label = gtk4::Label::new(Some("Size:"));
        size_label.set_margin_start(8);
        container.append(&size_label);

        let size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        size_spin.set_value(initial_size);
        container.append(&size_spin);

        // Connect theme button handlers - sets font family from theme, keeps current size
        for (i, btn) in theme_buttons.iter().enumerate() {
            let theme_index_clone = theme_index.clone();
            let theme_config_clone = theme_config.clone();
            let font_button_clone = font_button.clone();
            let custom_family_clone = custom_family.clone();
            let on_change_clone = on_change.clone();
            let size_spin_clone = size_spin.clone();

            btn.connect_toggled(move |toggle_btn| {
                if toggle_btn.is_active() {
                    let idx = (i + 1) as u8;
                    *theme_index_clone.borrow_mut() = Some(idx);

                    // Get theme font family and update button label
                    if let Some(ref cfg) = *theme_config_clone.borrow() {
                        let (family, _) = cfg.get_font(idx);
                        font_button_clone.set_label(&family);
                        *custom_family_clone.borrow_mut() = family;
                    }

                    // Emit change with Theme font source including current size
                    if let Some(ref callback) = *on_change_clone.borrow() {
                        callback(FontSource::Theme { index: idx, size: size_spin_clone.value() });
                    }
                }
            });
        }

        // Font button - pick custom font (deselects theme)
        let theme_index_for_click = theme_index.clone();
        let custom_family_for_click = custom_family.clone();
        let on_change_for_click = on_change.clone();
        let theme_buttons_for_click: Vec<ToggleButton> = theme_buttons.to_vec();
        let font_button_for_click = font_button.clone();
        let size_spin_for_click = size_spin.clone();

        font_button.connect_clicked(move |btn| {
            let current_family = custom_family_for_click.borrow().clone();
            let window = btn
                .root()
                .and_then(|root| root.downcast::<gtk4::Window>().ok());

            if let Some(window) = window {
                let font_desc = gtk4::pango::FontDescription::from_string(&current_family);
                let theme_index_clone2 = theme_index_for_click.clone();
                let custom_family_clone2 = custom_family_for_click.clone();
                let on_change_clone2 = on_change_for_click.clone();
                let theme_buttons_clone2 = theme_buttons_for_click.clone();
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
                            *theme_index_clone2.borrow_mut() = None;

                            // Deselect theme buttons
                            for btn in &theme_buttons_clone2 {
                                btn.set_active(false);
                            }
                            font_button_clone2.set_label(&family);

                            if let Some(ref callback) = *on_change_clone2.borrow() {
                                callback(FontSource::Custom {
                                    family,
                                    size: size_spin_clone2.value(),
                                });
                            }
                        }
                    },
                );
            }
        });

        // Size spinner - emits change preserving current font source type
        let custom_family_for_spin = custom_family.clone();
        let theme_index_for_spin = theme_index.clone();
        let on_change_for_spin = on_change.clone();

        size_spin.connect_value_changed(move |spin| {
            if let Some(ref callback) = *on_change_for_spin.borrow() {
                // Preserve theme selection, update size
                if let Some(idx) = *theme_index_for_spin.borrow() {
                    callback(FontSource::Theme { index: idx, size: spin.value() });
                } else {
                    callback(FontSource::Custom {
                        family: custom_family_for_spin.borrow().clone(),
                        size: spin.value(),
                    });
                }
            }
        });

        Self {
            container,
            theme_buttons,
            font_button,
            size_spin,
            theme_index,
            custom_family,
            theme_config,
            on_change,
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn source(&self) -> FontSource {
        if let Some(idx) = *self.theme_index.borrow() {
            FontSource::Theme { index: idx, size: self.size_spin.value() }
        } else {
            FontSource::Custom {
                family: self.custom_family.borrow().clone(),
                size: self.size_spin.value(),
            }
        }
    }

    pub fn set_source(&self, source: FontSource) {
        match &source {
            FontSource::Theme { index, size } => {
                let idx = (*index as usize).saturating_sub(1).min(1);
                self.theme_buttons[idx].set_active(true);
                *self.theme_index.borrow_mut() = Some(*index);
                self.size_spin.set_value(*size);

                if let Some(ref cfg) = *self.theme_config.borrow() {
                    let (family, _) = cfg.get_font(*index);
                    self.font_button.set_label(&family);
                    *self.custom_family.borrow_mut() = family;
                }
            }
            FontSource::Custom { family, size } => {
                for btn in &self.theme_buttons {
                    btn.set_active(false);
                }
                *self.theme_index.borrow_mut() = None;
                *self.custom_family.borrow_mut() = family.clone();
                self.font_button.set_label(family);
                self.size_spin.set_value(*size);
            }
        }
    }

    pub fn set_theme_config(&self, config: ComboThemeConfig) {
        *self.theme_config.borrow_mut() = Some(config.clone());

        // Update T1/T2 button tooltips with actual theme font names
        let (font1_family, font1_size) = config.get_font(1);
        let (font2_family, font2_size) = config.get_font(2);
        self.theme_buttons[0].set_tooltip_text(Some(&format!("Theme Font 1: {} {:.0}pt", font1_family, font1_size)));
        self.theme_buttons[1].set_tooltip_text(Some(&format!("Theme Font 2: {} {:.0}pt", font2_family, font2_size)));

        // Update T1/T2 button labels to show font name abbreviation
        let abbrev1 = if font1_family.len() > 8 { &font1_family[..8] } else { &font1_family };
        let abbrev2 = if font2_family.len() > 8 { &font2_family[..8] } else { &font2_family };
        self.theme_buttons[0].set_label(&format!("T1:{}", abbrev1));
        self.theme_buttons[1].set_label(&format!("T2:{}", abbrev2));

        // Update font button label if using theme
        if let Some(idx) = *self.theme_index.borrow() {
            let (family, _) = config.get_font(idx);
            self.font_button.set_label(&family);
            *self.custom_family.borrow_mut() = family;
        }
    }

    pub fn set_on_change<F: Fn(FontSource) + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Get current font family and size
    pub fn resolve_font(&self) -> (String, f64) {
        (self.custom_family.borrow().clone(), self.size_spin.value())
    }

    /// Get the current font size (independent of theme selection)
    pub fn size(&self) -> f64 {
        self.size_spin.value()
    }

    /// Set the font size without affecting theme selection
    pub fn set_size(&self, size: f64) {
        self.size_spin.set_value(size);
    }
}
