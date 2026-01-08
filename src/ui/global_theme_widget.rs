//! Widget for configuring the global theme (for non-combo panels)

use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::shared_font_dialog::show_font_dialog;
use crate::ui::theme::ComboThemeConfig;
use crate::ui::GradientEditor;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Grid, Label, Orientation, SpinButton, Widget};
use std::cell::RefCell;
use std::rc::Rc;

/// Widget for configuring the global theme
pub struct GlobalThemeWidget {
    widget: GtkBox,
    config: Rc<RefCell<ComboThemeConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    // Color widgets
    color1_widget: Rc<ColorButtonWidget>,
    color2_widget: Rc<ColorButtonWidget>,
    color3_widget: Rc<ColorButtonWidget>,
    color4_widget: Rc<ColorButtonWidget>,
    // Gradient editor
    gradient_editor: Rc<GradientEditor>,
    // Font controls
    font1_btn: Button,
    font1_size_spin: SpinButton,
    font2_btn: Button,
    font2_size_spin: SpinButton,
}

impl GlobalThemeWidget {
    /// Create a new global theme widget
    pub fn new() -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 8);
        widget.set_margin_start(12);
        widget.set_margin_end(12);
        widget.set_margin_top(12);
        widget.set_margin_bottom(12);

        let config = Rc::new(RefCell::new(ComboThemeConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Theme Colors section
        let colors_label = Label::new(Some("Theme Colors"));
        colors_label.set_halign(gtk4::Align::Start);
        colors_label.add_css_class("heading");
        widget.append(&colors_label);

        // Use a Grid for aligned color rows
        let colors_grid = Grid::new();
        colors_grid.set_row_spacing(6);
        colors_grid.set_column_spacing(12);
        colors_grid.set_margin_start(8);

        // Color 1 (Primary)
        let color1_label = Label::new(Some("Color 1 (Primary):"));
        color1_label.set_halign(gtk4::Align::End);
        colors_grid.attach(&color1_label, 0, 0, 1, 1);
        let color1_widget = Rc::new(ColorButtonWidget::new(config.borrow().color1));
        colors_grid.attach(color1_widget.widget(), 1, 0, 1, 1);

        // Color 2 (Secondary)
        let color2_label = Label::new(Some("Color 2 (Secondary):"));
        color2_label.set_halign(gtk4::Align::End);
        colors_grid.attach(&color2_label, 0, 1, 1, 1);
        let color2_widget = Rc::new(ColorButtonWidget::new(config.borrow().color2));
        colors_grid.attach(color2_widget.widget(), 1, 1, 1, 1);

        // Color 3 (Accent)
        let color3_label = Label::new(Some("Color 3 (Accent):"));
        color3_label.set_halign(gtk4::Align::End);
        colors_grid.attach(&color3_label, 0, 2, 1, 1);
        let color3_widget = Rc::new(ColorButtonWidget::new(config.borrow().color3));
        colors_grid.attach(color3_widget.widget(), 1, 2, 1, 1);

        // Color 4 (Highlight)
        let color4_label = Label::new(Some("Color 4 (Highlight):"));
        color4_label.set_halign(gtk4::Align::End);
        colors_grid.attach(&color4_label, 0, 3, 1, 1);
        let color4_widget = Rc::new(ColorButtonWidget::new(config.borrow().color4));
        colors_grid.attach(color4_widget.widget(), 1, 3, 1, 1);

        widget.append(&colors_grid);

        // Theme Gradient section
        let gradient_label = Label::new(Some("Theme Gradient"));
        gradient_label.set_halign(gtk4::Align::Start);
        gradient_label.add_css_class("heading");
        gradient_label.set_margin_top(12);
        widget.append(&gradient_label);

        let gradient_editor = Rc::new(GradientEditor::new());
        gradient_editor.set_gradient_source_config(&config.borrow().gradient);
        // Set initial theme config for the gradient editor
        gradient_editor.set_theme_config(config.borrow().clone());
        widget.append(gradient_editor.widget());

        // Helper to update gradient editor theme when colors change
        let update_gradient_theme = {
            let config_for_theme = config.clone();
            let gradient_editor_for_theme = gradient_editor.clone();
            Rc::new(move || {
                gradient_editor_for_theme.set_theme_config(config_for_theme.borrow().clone());
            })
        };

        // Connect color widget callbacks
        {
            let config_c1 = config.clone();
            let on_change_c1 = on_change.clone();
            let update_theme_c1 = update_gradient_theme.clone();
            color1_widget.set_on_change(move |color| {
                config_c1.borrow_mut().color1 = color;
                update_theme_c1();
                if let Some(cb) = on_change_c1.borrow().as_ref() {
                    cb();
                }
            });
        }
        {
            let config_c2 = config.clone();
            let on_change_c2 = on_change.clone();
            let update_theme_c2 = update_gradient_theme.clone();
            color2_widget.set_on_change(move |color| {
                config_c2.borrow_mut().color2 = color;
                update_theme_c2();
                if let Some(cb) = on_change_c2.borrow().as_ref() {
                    cb();
                }
            });
        }
        {
            let config_c3 = config.clone();
            let on_change_c3 = on_change.clone();
            let update_theme_c3 = update_gradient_theme.clone();
            color3_widget.set_on_change(move |color| {
                config_c3.borrow_mut().color3 = color;
                update_theme_c3();
                if let Some(cb) = on_change_c3.borrow().as_ref() {
                    cb();
                }
            });
        }
        {
            let config_c4 = config.clone();
            let on_change_c4 = on_change.clone();
            let update_theme_c4 = update_gradient_theme.clone();
            color4_widget.set_on_change(move |color| {
                config_c4.borrow_mut().color4 = color;
                update_theme_c4();
                if let Some(cb) = on_change_c4.borrow().as_ref() {
                    cb();
                }
            });
        }

        // Connect gradient editor on_change
        {
            let config_grad = config.clone();
            let on_change_grad = on_change.clone();
            let gradient_editor_clone = gradient_editor.clone();
            gradient_editor.set_on_change(move || {
                config_grad.borrow_mut().gradient = gradient_editor_clone.get_gradient_source_config();
                if let Some(cb) = on_change_grad.borrow().as_ref() {
                    cb();
                }
            });
        }

        // Theme Fonts section
        let fonts_label = Label::new(Some("Theme Fonts"));
        fonts_label.set_halign(gtk4::Align::Start);
        fonts_label.add_css_class("heading");
        fonts_label.set_margin_top(12);
        widget.append(&fonts_label);

        // Font 1
        let font1_box = GtkBox::new(Orientation::Horizontal, 6);
        font1_box.append(&Label::new(Some("Font 1:")));
        let font1_btn = Button::with_label(&config.borrow().font1_family);
        font1_btn.set_hexpand(true);
        font1_box.append(&font1_btn);
        font1_box.append(&Label::new(Some("Size:")));
        let font1_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        font1_size_spin.set_value(config.borrow().font1_size);
        font1_box.append(&font1_size_spin);
        widget.append(&font1_box);

        // Font 1 button click handler
        {
            let config_f1 = config.clone();
            let on_change_f1 = on_change.clone();
            let font1_btn_clone = font1_btn.clone();
            font1_btn.connect_clicked(move |button| {
                let config_for_cb = config_f1.clone();
                let on_change_for_cb = on_change_f1.clone();
                let font_btn_for_cb = font1_btn_clone.clone();
                if let Some(window) = button.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) {
                    let current_font = config_for_cb.borrow().font1_family.clone();
                    let font_desc = gtk4::pango::FontDescription::from_string(&current_font);
                    show_font_dialog(Some(&window), Some(&font_desc), move |font_desc| {
                        let family = font_desc.family()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "sans-serif".to_string());
                        config_for_cb.borrow_mut().font1_family = family.clone();
                        font_btn_for_cb.set_label(&family);
                        if let Some(cb) = on_change_for_cb.borrow().as_ref() {
                            cb();
                        }
                    });
                }
            });
        }

        // Font 1 size spin handler
        {
            let config_f1s = config.clone();
            let on_change_f1s = on_change.clone();
            font1_size_spin.connect_value_changed(move |spin| {
                config_f1s.borrow_mut().font1_size = spin.value();
                if let Some(cb) = on_change_f1s.borrow().as_ref() {
                    cb();
                }
            });
        }

        // Font 2
        let font2_box = GtkBox::new(Orientation::Horizontal, 6);
        font2_box.append(&Label::new(Some("Font 2:")));
        let font2_btn = Button::with_label(&config.borrow().font2_family);
        font2_btn.set_hexpand(true);
        font2_box.append(&font2_btn);
        font2_box.append(&Label::new(Some("Size:")));
        let font2_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        font2_size_spin.set_value(config.borrow().font2_size);
        font2_box.append(&font2_size_spin);
        widget.append(&font2_box);

        // Font 2 button click handler
        {
            let config_f2 = config.clone();
            let on_change_f2 = on_change.clone();
            let font2_btn_clone = font2_btn.clone();
            font2_btn.connect_clicked(move |button| {
                let config_for_cb = config_f2.clone();
                let on_change_for_cb = on_change_f2.clone();
                let font_btn_for_cb = font2_btn_clone.clone();
                if let Some(window) = button.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) {
                    let current_font = config_for_cb.borrow().font2_family.clone();
                    let font_desc = gtk4::pango::FontDescription::from_string(&current_font);
                    show_font_dialog(Some(&window), Some(&font_desc), move |font_desc| {
                        let family = font_desc.family()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "sans-serif".to_string());
                        config_for_cb.borrow_mut().font2_family = family.clone();
                        font_btn_for_cb.set_label(&family);
                        if let Some(cb) = on_change_for_cb.borrow().as_ref() {
                            cb();
                        }
                    });
                }
            });
        }

        // Font 2 size spin handler
        {
            let config_f2s = config.clone();
            let on_change_f2s = on_change.clone();
            font2_size_spin.connect_value_changed(move |spin| {
                config_f2s.borrow_mut().font2_size = spin.value();
                if let Some(cb) = on_change_f2s.borrow().as_ref() {
                    cb();
                }
            });
        }

        Self {
            widget,
            config,
            on_change,
            color1_widget,
            color2_widget,
            color3_widget,
            color4_widget,
            gradient_editor,
            font1_btn,
            font1_size_spin,
            font2_btn,
            font2_size_spin,
        }
    }

    /// Get the widget
    pub fn widget(&self) -> &Widget {
        self.widget.upcast_ref()
    }

    /// Set the configuration
    pub fn set_config(&self, new_config: ComboThemeConfig) {
        *self.config.borrow_mut() = new_config.clone();

        // Update color widgets
        self.color1_widget.set_color(new_config.color1);
        self.color2_widget.set_color(new_config.color2);
        self.color3_widget.set_color(new_config.color3);
        self.color4_widget.set_color(new_config.color4);

        // Update gradient editor with theme and config
        self.gradient_editor.set_theme_config(new_config.clone());
        self.gradient_editor.set_gradient_source_config(&new_config.gradient);

        // Update font buttons and size spins
        self.font1_btn.set_label(&new_config.font1_family);
        self.font1_size_spin.set_value(new_config.font1_size);
        self.font2_btn.set_label(&new_config.font2_family);
        self.font2_size_spin.set_value(new_config.font2_size);
    }

    /// Get the current configuration
    pub fn get_config(&self) -> ComboThemeConfig {
        self.config.borrow().clone()
    }

    /// Set the on_change callback
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }
}

impl Default for GlobalThemeWidget {
    fn default() -> Self {
        Self::new()
    }
}
