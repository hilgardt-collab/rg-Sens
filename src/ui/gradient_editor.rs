//! Gradient editor widget for configuring linear and radial gradients

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, DrawingArea, Label, ListBox, ListBoxRow, Orientation, Scale, SpinButton};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::background::{Color, ColorStop, LinearGradientConfig};
use crate::ui::render_utils::render_checkerboard;
use crate::ui::theme::{ColorStopSource, ComboThemeConfig, LinearGradientSourceConfig};
use crate::ui::theme_color_selector::ThemeColorSelector;

/// Gradient editor widget
pub struct GradientEditor {
    container: GtkBox,
    stops: Rc<RefCell<Vec<ColorStopSource>>>,
    angle: Rc<RefCell<f64>>,
    theme_config: Rc<RefCell<Option<ComboThemeConfig>>>,
    on_change: Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
    preview: DrawingArea,
    stops_listbox: ListBox,
    angle_scale: Option<Scale>,
    angle_spin: Option<SpinButton>,
    /// Guard flag to prevent infinite callback loops during theme updates
    is_updating: Rc<RefCell<bool>>,
}

impl GradientEditor {
    pub fn new() -> Self {
        Self::new_with_options(true, true)
    }

    /// Create a gradient editor without the angle control (for radial gradients)
    pub fn new_without_angle() -> Self {
        Self::new_with_options(false, false)
    }

    /// Create a gradient editor with linear preview but no angle control (for value mapping)
    pub fn new_linear_no_angle() -> Self {
        Self::new_with_options(false, true)
    }

    fn new_with_options(show_angle: bool, use_linear_preview: bool) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);
        container.set_vexpand(true);

        let stops: Rc<RefCell<Vec<ColorStopSource>>> = Rc::new(RefCell::new(Vec::new()));
        let angle = Rc::new(RefCell::new(0.0));
        let theme_config: Rc<RefCell<Option<ComboThemeConfig>>> = Rc::new(RefCell::new(None));
        let on_change: Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let is_updating: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

        // Preview area (created early so angle handlers can reference it)
        let preview = DrawingArea::new();
        preview.set_content_height(100); // Keep gradient preview shorter
        preview.set_content_width(200); // Min width 200px
        preview.set_hexpand(true);
        preview.set_halign(gtk4::Align::Fill); // Fill available width
        preview.set_vexpand(false);

        let stops_clone = stops.clone();
        let angle_clone = angle.clone();
        let theme_config_clone = theme_config.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            use crate::ui::background::render_background;
            use crate::ui::background::{BackgroundConfig, BackgroundType, LinearGradientConfig, RadialGradientConfig};

            // Render checkerboard pattern to show transparency
            render_checkerboard(cr, width as f64, height as f64);

            let stops_source = stops_clone.borrow();
            let angle = *angle_clone.borrow();

            // Resolve ColorStopSource to ColorStop using theme
            let default_theme = ComboThemeConfig::default();
            let theme = theme_config_clone.borrow();
            let theme_ref = theme.as_ref().unwrap_or(&default_theme);
            let resolved_stops: Vec<ColorStop> = stops_source.iter()
                .map(|s| s.resolve(theme_ref))
                .collect();

            let config = if use_linear_preview {
                BackgroundConfig {
                    background: BackgroundType::LinearGradient(LinearGradientConfig {
                        angle,
                        stops: resolved_stops,
                    }),
                }
            } else {
                BackgroundConfig {
                    background: BackgroundType::RadialGradient(RadialGradientConfig {
                        center_x: 0.5,
                        center_y: 0.5,
                        radius: 0.7,
                        stops: resolved_stops,
                    }),
                }
            };

            let _ = render_background(cr, &config, width as f64, height as f64);
        });

        // Angle control (only if show_angle is true)
        let (angle_scale, angle_spin) = if show_angle {
            let angle_box = GtkBox::new(Orientation::Horizontal, 6);
            angle_box.append(&Label::new(Some("Angle:")));

            let angle_scale = Scale::with_range(Orientation::Horizontal, -360.0, 360.0, 1.0);
            angle_scale.set_hexpand(true);
            angle_scale.set_value(0.0);

            let angle_spin = SpinButton::with_range(-360.0, 360.0, 1.0);
            angle_spin.set_value(0.0);
            angle_spin.set_digits(0);

            // Sync scale and spin button
            let angle_clone = angle.clone();
            let angle_spin_clone = angle_spin.clone();
            let on_change_clone = on_change.clone();
            let preview_clone = preview.clone();
            let is_updating_clone = is_updating.clone();
            angle_scale.connect_value_changed(move |scale| {
                // Skip if we're already updating (prevents infinite loop)
                if *is_updating_clone.borrow() {
                    return;
                }

                let value = scale.value();
                // Set guard before updating spin to prevent feedback loop
                *is_updating_clone.borrow_mut() = true;
                angle_spin_clone.set_value(value);
                *is_updating_clone.borrow_mut() = false;

                *angle_clone.borrow_mut() = value;
                preview_clone.queue_draw();
                if let Some(callback) = on_change_clone.borrow().as_ref() {
                    callback();
                }
            });

            let angle_scale_clone = angle_scale.clone();
            let angle_clone2 = angle.clone();
            let on_change_clone2 = on_change.clone();
            let preview_clone2 = preview.clone();
            let is_updating_clone2 = is_updating.clone();
            angle_spin.connect_value_changed(move |spin| {
                // Skip if we're already updating (prevents infinite loop)
                if *is_updating_clone2.borrow() {
                    return;
                }

                let value = spin.value();
                // Set guard before updating scale to prevent feedback loop
                *is_updating_clone2.borrow_mut() = true;
                angle_scale_clone.set_value(value);
                *is_updating_clone2.borrow_mut() = false;

                *angle_clone2.borrow_mut() = value;
                preview_clone2.queue_draw();
                if let Some(callback) = on_change_clone2.borrow().as_ref() {
                    callback();
                }
            });

            angle_box.append(&angle_scale);
            angle_box.append(&angle_spin);
            container.append(&angle_box);

            (Some(angle_scale), Some(angle_spin))
        } else {
            (None, None)
        };

        container.append(&preview);

        // Color stops header with Add button
        let header_box = GtkBox::new(Orientation::Horizontal, 6);
        let stops_label = Label::new(Some("Color Stops:"));
        stops_label.set_halign(gtk4::Align::Start);
        stops_label.set_hexpand(true);
        header_box.append(&stops_label);

        let add_button = Button::with_label("Add Stop");
        header_box.append(&add_button);
        container.append(&header_box);

        // Stops list
        let stops_listbox = ListBox::new();
        stops_listbox.set_selection_mode(gtk4::SelectionMode::None);
        stops_listbox.add_css_class("boxed-list");

        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_child(Some(&stops_listbox));
        scroll.set_vexpand(true);
        // Allow the scroll window to grow with content up to a reasonable size
        scroll.set_propagate_natural_height(true);
        scroll.set_max_content_height(300);
        scroll.set_min_content_height(80);
        container.append(&scroll);

        // Add stop button handler
        let stops_clone = stops.clone();
        let stops_listbox_clone = stops_listbox.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        let is_updating_clone = is_updating.clone();

        let theme_config_for_add = theme_config.clone();
        add_button.connect_clicked(move |_| {
            // Skip if we're already updating (prevents infinite loop)
            if *is_updating_clone.borrow() {
                return;
            }

            let mut stops_list = stops_clone.borrow_mut();

            // Find a good position for the new stop
            let position = if stops_list.is_empty() {
                0.5
            } else {
                let mut positions: Vec<f64> = stops_list.iter().map(|s| s.position).collect();
                positions.sort_by(|a: &f64, b: &f64| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                let mut max_gap = positions[0];
                let mut max_gap_pos = positions[0] / 2.0;

                for i in 0..positions.len() - 1 {
                    let gap = positions[i + 1] - positions[i];
                    if gap > max_gap {
                        max_gap = gap;
                        max_gap_pos = (positions[i] + positions[i + 1]) / 2.0;
                    }
                }

                if 1.0 - positions.last().unwrap() > max_gap {
                    (1.0 + positions.last().unwrap()) / 2.0
                } else {
                    max_gap_pos
                }
            };

            // New stops default to custom gray color
            let new_stop = ColorStopSource::custom(position, Color::new(0.5, 0.5, 0.5, 1.0));
            stops_list.push(new_stop);
            stops_list.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap_or(std::cmp::Ordering::Equal));

            drop(stops_list);

            Self::rebuild_stops_list(
                &stops_listbox_clone,
                &stops_clone,
                &preview_clone,
                &on_change_clone,
                &theme_config_for_add,
                &is_updating_clone,
            );

            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        Self {
            container,
            stops,
            angle,
            theme_config,
            on_change,
            preview,
            stops_listbox,
            angle_scale,
            angle_spin,
            is_updating,
        }
    }

    /// Rebuild the stops list UI
    fn rebuild_stops_list(
        listbox: &ListBox,
        stops: &Rc<RefCell<Vec<ColorStopSource>>>,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
        theme_config: &Rc<RefCell<Option<ComboThemeConfig>>>,
        is_updating: &Rc<RefCell<bool>>,
    ) {
        // Clear existing rows
        while let Some(child) = listbox.first_child() {
            listbox.remove(&child);
        }

        let stops_ref = stops.borrow();
        let stop_count = stops_ref.len();

        for (index, stop) in stops_ref.iter().enumerate() {
            let row = Self::create_stop_row(
                index,
                stop,
                stop_count,
                stops,
                listbox,
                preview,
                on_change,
                theme_config,
                is_updating,
            );
            listbox.append(&row);
        }
    }

    /// Create a row for a color stop
    fn create_stop_row(
        index: usize,
        stop: &ColorStopSource,
        stop_count: usize,
        stops: &Rc<RefCell<Vec<ColorStopSource>>>,
        listbox: &ListBox,
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<std::boxed::Box<dyn Fn()>>>>,
        theme_config: &Rc<RefCell<Option<ComboThemeConfig>>>,
        is_updating: &Rc<RefCell<bool>>,
    ) -> ListBoxRow {
        let row = ListBoxRow::new();
        let hbox = GtkBox::new(Orientation::Horizontal, 12);
        hbox.set_margin_start(12);
        hbox.set_margin_end(12);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);

        // Position spinner
        let position_box = GtkBox::new(Orientation::Horizontal, 6);
        let position_label = Label::new(Some("Position:"));
        position_label.set_halign(gtk4::Align::Start);

        let position_spin = SpinButton::with_range(0.0, 100.0, 1.0);
        position_spin.set_value(stop.position * 100.0); // Convert to percentage
        position_spin.set_digits(0);
        position_spin.set_width_request(80);

        let percent_label = Label::new(Some("%"));

        position_box.append(&position_label);
        position_box.append(&position_spin);
        position_box.append(&percent_label);
        hbox.append(&position_box);

        // Color selector using ThemeColorSelector
        let color_selector = ThemeColorSelector::new(stop.color.clone());
        if let Some(ref cfg) = *theme_config.borrow() {
            color_selector.set_theme_config(cfg.clone());
        }
        hbox.append(color_selector.widget());

        // Set up color change handler
        let stops_clone = stops.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        let is_updating_clone = is_updating.clone();
        color_selector.set_on_change(move |new_color_source| {
            // Skip if we're already updating (prevents infinite loop)
            if *is_updating_clone.borrow() {
                return;
            }

            {
                let mut stops = stops_clone.borrow_mut();
                if let Some(stop) = stops.get_mut(index) {
                    stop.color = new_color_source;
                }
            }

            // No need to rebuild the stops list - just update the data and redraw
            // The ThemeColorSelector already displays the new color
            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        // Remove button (only if more than 2 stops)
        if stop_count > 2 {
            let remove_button = Button::from_icon_name("user-trash-symbolic");
            remove_button.set_tooltip_text(Some("Remove stop"));

            let stops_clone = stops.clone();
            let listbox_clone = listbox.clone();
            let preview_clone = preview.clone();
            let on_change_clone = on_change.clone();
            let theme_config_clone = theme_config.clone();
            let is_updating_clone = is_updating.clone();

            remove_button.connect_clicked(move |_| {
                // Skip if we're already updating (prevents infinite loop)
                if *is_updating_clone.borrow() {
                    return;
                }

                let mut stops = stops_clone.borrow_mut();
                if stops.len() > 2 {
                    stops.remove(index);
                    drop(stops);

                    Self::rebuild_stops_list(
                        &listbox_clone,
                        &stops_clone,
                        &preview_clone,
                        &on_change_clone,
                        &theme_config_clone,
                        &is_updating_clone,
                    );

                    preview_clone.queue_draw();

                    if let Some(callback) = on_change_clone.borrow().as_ref() {
                        callback();
                    }
                }
            });

            hbox.append(&remove_button);
        }

        row.set_child(Some(&hbox));

        // Position change handler
        let stops_clone = stops.clone();
        let listbox_clone = listbox.clone();
        let preview_clone = preview.clone();
        let on_change_clone = on_change.clone();
        let theme_config_clone = theme_config.clone();
        let is_updating_clone = is_updating.clone();

        position_spin.connect_value_changed(move |spin| {
            // Skip if we're already updating (prevents infinite loop)
            if *is_updating_clone.borrow() {
                return;
            }

            let mut new_position = spin.value() / 100.0; // Convert from percentage to 0.0-1.0

            // Validate: ensure minimum spacing of 0.01 (1%) between adjacent stops
            const MIN_SPACING: f64 = 0.01;

            let needs_rebuild;
            {
                let stops = stops_clone.borrow();
                // Check if this position would be too close to another stop
                for (i, other_stop) in stops.iter().enumerate() {
                    if i != index {
                        let distance = (new_position - other_stop.position).abs();
                        if distance < MIN_SPACING && distance > 0.0 {
                            // Adjust position to maintain minimum spacing
                            if new_position < other_stop.position {
                                new_position = (other_stop.position - MIN_SPACING).max(0.0);
                            } else {
                                new_position = (other_stop.position + MIN_SPACING).min(1.0);
                            }
                        }
                    }
                }

                // Check if order would change (needs rebuild)
                let old_index = index;
                let would_be_index = stops.iter()
                    .enumerate()
                    .filter(|(i, _)| *i != index)
                    .filter(|(_, s)| s.position < new_position)
                    .count();
                needs_rebuild = would_be_index != old_index.min(stops.len().saturating_sub(1));
            }

            {
                let mut stops = stops_clone.borrow_mut();
                if let Some(stop) = stops.get_mut(index) {
                    stop.position = new_position;
                }
                stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap_or(std::cmp::Ordering::Equal));
            }

            // Only rebuild the list if the order changed - defer to idle to avoid
            // GTK adjustment issues when the SpinButton is still being interacted with
            if needs_rebuild {
                let listbox_clone2 = listbox_clone.clone();
                let stops_clone2 = stops_clone.clone();
                let preview_clone2 = preview_clone.clone();
                let on_change_clone2 = on_change_clone.clone();
                let theme_config_clone2 = theme_config_clone.clone();
                let is_updating_clone2 = is_updating_clone.clone();
                gtk4::glib::idle_add_local_once(move || {
                    Self::rebuild_stops_list(
                        &listbox_clone2,
                        &stops_clone2,
                        &preview_clone2,
                        &on_change_clone2,
                        &theme_config_clone2,
                        &is_updating_clone2,
                    );
                });
            }

            preview_clone.queue_draw();

            if let Some(callback) = on_change_clone.borrow().as_ref() {
                callback();
            }
        });

        row
    }

    /// Set the theme configuration for resolving theme colors
    pub fn set_theme_config(&self, config: ComboThemeConfig) {
        // Check if theme actually changed to avoid unnecessary rebuilds
        let theme_changed = {
            let current = self.theme_config.borrow();
            match current.as_ref() {
                Some(current_config) => {
                    // Compare theme colors - if they're the same, no need to rebuild
                    current_config.color1 != config.color1 ||
                    current_config.color2 != config.color2 ||
                    current_config.color3 != config.color3 ||
                    current_config.color4 != config.color4
                }
                None => true, // No current theme, need to set it
            }
        };

        // Always update the theme config for preview rendering
        *self.theme_config.borrow_mut() = Some(config);
        self.preview.queue_draw();

        // Only rebuild stops list if theme colors actually changed
        if theme_changed {
            // Set guard to prevent callbacks from firing during theme update
            *self.is_updating.borrow_mut() = true;

            // Rebuild stops list to update ThemeColorSelectors with new theme
            Self::rebuild_stops_list(
                &self.stops_listbox,
                &self.stops,
                &self.preview,
                &self.on_change,
                &self.theme_config,
                &self.is_updating,
            );

            // Clear guard
            *self.is_updating.borrow_mut() = false;
        }
    }

    /// Set the gradient configuration using ColorStopSource (theme-aware)
    pub fn set_stops_source(&self, stops: Vec<ColorStopSource>) {
        // Set guard to prevent callbacks from firing during update
        *self.is_updating.borrow_mut() = true;

        *self.stops.borrow_mut() = stops;

        Self::rebuild_stops_list(
            &self.stops_listbox,
            &self.stops,
            &self.preview,
            &self.on_change,
            &self.theme_config,
            &self.is_updating,
        );
        self.preview.queue_draw();

        // Clear guard
        *self.is_updating.borrow_mut() = false;
    }

    /// Get the color stops as ColorStopSource (theme-aware)
    pub fn get_stops_source(&self) -> Vec<ColorStopSource> {
        self.stops.borrow().clone()
    }

    /// Set angle and stops using ColorStopSource
    pub fn set_gradient_source(&self, angle: f64, stops: Vec<ColorStopSource>) {
        // Set guard to prevent callbacks from firing during update
        *self.is_updating.borrow_mut() = true;

        *self.stops.borrow_mut() = stops;
        *self.angle.borrow_mut() = angle;

        // Update the angle UI widgets (if they exist)
        if let Some(ref angle_scale) = self.angle_scale {
            angle_scale.set_value(angle);
        }
        if let Some(ref angle_spin) = self.angle_spin {
            angle_spin.set_value(angle);
        }

        Self::rebuild_stops_list(
            &self.stops_listbox,
            &self.stops,
            &self.preview,
            &self.on_change,
            &self.theme_config,
            &self.is_updating,
        );
        self.preview.queue_draw();

        // Clear guard
        *self.is_updating.borrow_mut() = false;
    }

    /// Set the gradient configuration (backward compatible - converts to Custom colors)
    pub fn set_gradient(&self, config: &LinearGradientConfig) {
        // Set guard to prevent callbacks from firing during update
        *self.is_updating.borrow_mut() = true;

        // Convert ColorStop to ColorStopSource with Custom colors
        let stops_source: Vec<ColorStopSource> = config.stops.iter()
            .map(|s| ColorStopSource::custom(s.position, s.color))
            .collect();

        *self.stops.borrow_mut() = stops_source;
        *self.angle.borrow_mut() = config.angle;

        // Update the angle UI widgets (if they exist)
        if let Some(ref angle_scale) = self.angle_scale {
            angle_scale.set_value(config.angle);
        }
        if let Some(ref angle_spin) = self.angle_spin {
            angle_spin.set_value(config.angle);
        }

        Self::rebuild_stops_list(
            &self.stops_listbox,
            &self.stops,
            &self.preview,
            &self.on_change,
            &self.theme_config,
            &self.is_updating,
        );
        self.preview.queue_draw();

        // Clear guard
        *self.is_updating.borrow_mut() = false;
    }

    /// Set just the color stops (backward compatible - converts to Custom colors)
    pub fn set_stops(&self, stops: Vec<ColorStop>) {
        // Set guard to prevent callbacks from firing during update
        *self.is_updating.borrow_mut() = true;

        // Convert ColorStop to ColorStopSource with Custom colors
        let stops_source: Vec<ColorStopSource> = stops.iter()
            .map(|s| ColorStopSource::custom(s.position, s.color))
            .collect();

        *self.stops.borrow_mut() = stops_source;

        Self::rebuild_stops_list(
            &self.stops_listbox,
            &self.stops,
            &self.preview,
            &self.on_change,
            &self.theme_config,
            &self.is_updating,
        );
        self.preview.queue_draw();

        // Clear guard
        *self.is_updating.borrow_mut() = false;
    }

    /// Get just the color stops (resolved to concrete colors)
    pub fn get_stops(&self) -> Vec<ColorStop> {
        let default_theme = ComboThemeConfig::default();
        let theme = self.theme_config.borrow();
        let theme_ref = theme.as_ref().unwrap_or(&default_theme);

        self.stops.borrow().iter()
            .map(|s| s.resolve(theme_ref))
            .collect()
    }

    /// Get the current gradient configuration (resolved to concrete colors)
    pub fn get_gradient(&self) -> LinearGradientConfig {
        LinearGradientConfig {
            angle: *self.angle.borrow(),
            stops: self.get_stops(),
        }
    }

    /// Set gradient from LinearGradientSourceConfig (preserves theme references)
    pub fn set_gradient_source_config(&self, config: &LinearGradientSourceConfig) {
        self.set_gradient_source(config.angle, config.stops.clone());
    }

    /// Get the current gradient as LinearGradientSourceConfig (preserves theme references)
    pub fn get_gradient_source_config(&self) -> LinearGradientSourceConfig {
        LinearGradientSourceConfig {
            angle: *self.angle.borrow(),
            stops: self.stops.borrow().clone(),
        }
    }

    /// Set callback for when gradient changes
    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(std::boxed::Box::new(callback));
    }

    /// Get the container widget
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Update preview
    pub fn update_preview(&self) {
        self.preview.queue_draw();
    }

    /// Update only the theme colors without replacing the entire theme config.
    /// This preserves the gradient and font settings while updating C1-C4.
    pub fn update_theme_colors(
        &self,
        color1: crate::ui::Color,
        color2: crate::ui::Color,
        color3: crate::ui::Color,
        color4: crate::ui::Color,
    ) {
        // Get current theme or create default
        let mut theme = self.theme_config.borrow().clone().unwrap_or_default();

        // Check if colors actually changed
        let colors_changed = theme.color1 != color1
            || theme.color2 != color2
            || theme.color3 != color3
            || theme.color4 != color4;

        if !colors_changed {
            return;
        }

        // Update only the colors
        theme.color1 = color1;
        theme.color2 = color2;
        theme.color3 = color3;
        theme.color4 = color4;

        // Store updated theme
        *self.theme_config.borrow_mut() = Some(theme);
        self.preview.queue_draw();

        // Rebuild stops list to update ThemeColorSelectors with new theme
        *self.is_updating.borrow_mut() = true;
        Self::rebuild_stops_list(
            &self.stops_listbox,
            &self.stops,
            &self.preview,
            &self.on_change,
            &self.theme_config,
            &self.is_updating,
        );
        *self.is_updating.borrow_mut() = false;
    }
}

impl Default for GradientEditor {
    fn default() -> Self {
        Self::new()
    }
}
