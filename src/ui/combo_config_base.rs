//! Shared base functionality for combo panel configuration widgets
//!
//! This module provides common types and helper functions used by all combo panel
//! config widgets (Synthwave, LCARS, Cyberpunk, Material, Industrial, RetroTerminal, FighterHUD).

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Label, Notebook, Orientation,
    ScrolledWindow, SpinButton, StringList,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::core::{FieldMetadata, FieldPurpose, FieldType};
use crate::ui::clipboard::CLIPBOARD;
use crate::ui::lcars_display::{ContentDisplayType, ContentItemConfig, SplitOrientation};
use crate::ui::theme::{ComboThemeConfig, FontSource};
use crate::ui::widget_builder::{create_page_container, DEFAULT_MARGIN};

// Re-export for convenience - combo config widgets can import from here
pub use crate::ui::widget_builder::{queue_redraw, notify_change, OnChangeCallback};
use crate::ui::{
    ArcConfigWidget, ColorButtonWidget, CoreBarsConfigWidget, GradientEditor, LazyBarConfigWidget,
    LazyGraphConfigWidget, LazyTextOverlayConfigWidget, SpeedometerConfigWidget, StaticConfigWidget,
};
use crate::ui::shared_font_dialog::shared_font_dialog;

/// Trait for combo panel frame configurations that support theming
pub trait ThemedFrameConfig {
    /// Get reference to the theme configuration
    fn theme(&self) -> &ComboThemeConfig;
    /// Get mutable reference to the theme configuration
    fn theme_mut(&mut self) -> &mut ComboThemeConfig;
    /// Get reference to content items
    fn content_items(&self) -> &HashMap<String, ContentItemConfig>;
    /// Get mutable reference to content items
    fn content_items_mut(&mut self) -> &mut HashMap<String, ContentItemConfig>;
}

/// Trait for combo panel frame configurations that support layout/grouping
pub trait LayoutFrameConfig {
    /// Get the number of groups
    fn group_count(&self) -> usize;
    /// Get reference to group size weights
    fn group_size_weights(&self) -> &Vec<f64>;
    /// Get mutable reference to group size weights
    fn group_size_weights_mut(&mut self) -> &mut Vec<f64>;
    /// Get reference to per-group item orientations
    fn group_item_orientations(&self) -> &Vec<SplitOrientation>;
    /// Get mutable reference to per-group item orientations
    fn group_item_orientations_mut(&mut self) -> &mut Vec<SplitOrientation>;
    /// Get the split orientation (used as default for item orientations)
    fn split_orientation(&self) -> SplitOrientation;
}

/// Transferable configuration that can be preserved when switching between combo panel types.
/// This excludes theme-specific settings (colors, fonts, frame styles) but includes
/// layout and content configuration.
#[derive(Debug, Clone, Default)]
pub struct TransferableComboConfig {
    /// Number of groups
    pub group_count: usize,
    /// Number of items in each group
    pub group_item_counts: Vec<u32>,
    /// Size weight for each group
    pub group_size_weights: Vec<f64>,
    /// Item orientation within each group
    pub group_item_orientations: Vec<SplitOrientation>,
    /// Layout orientation (how groups are arranged)
    pub layout_orientation: SplitOrientation,
    /// Content items configuration (keyed by slot name like "group1_1")
    pub content_items: HashMap<String, ContentItemConfig>,
    /// Content padding
    pub content_padding: f64,
    /// Item spacing within groups
    pub item_spacing: f64,
    /// Animation enabled
    pub animation_enabled: bool,
    /// Animation speed
    pub animation_speed: f64,
}

impl TransferableComboConfig {
    /// Check if this config has meaningful content to transfer
    pub fn has_content(&self) -> bool {
        self.group_count > 0 || !self.content_items.is_empty()
    }
}

/// Common layout widgets found across combo panels
#[allow(dead_code)]
pub struct CommonLayoutWidgets {
    pub split_orientation_dropdown: DropDown,
    pub content_padding_spin: SpinButton,
    pub item_spacing_spin: SpinButton,
    pub divider_padding_spin: SpinButton,
    pub group_weights_box: GtkBox,
    pub item_orientations_box: GtkBox,
}

/// Common animation widgets found across combo panels
#[allow(dead_code)]
pub struct CommonAnimationWidgets {
    pub enable_check: CheckButton,
    pub speed_spin: SpinButton,
}

/// Common theme widgets shared across all combo panel configs.
///
/// This struct contains the 9 widgets that appear in every combo panel's theme tab:
/// - 4 color button widgets (C1-C4)
/// - 1 gradient editor
/// - 2 font buttons + 2 font size spinners
#[derive(Clone)]
pub struct CommonThemeWidgets {
    pub color1_widget: Rc<ColorButtonWidget>,
    pub color2_widget: Rc<ColorButtonWidget>,
    pub color3_widget: Rc<ColorButtonWidget>,
    pub color4_widget: Rc<ColorButtonWidget>,
    pub gradient_editor: Rc<GradientEditor>,
    pub font1_btn: Button,
    pub font1_size_spin: SpinButton,
    pub font2_btn: Button,
    pub font2_size_spin: SpinButton,
}

/// Set standard margins on a page box
pub fn set_page_margins(page: &GtkBox) {
    page.set_margin_start(DEFAULT_MARGIN);
    page.set_margin_end(DEFAULT_MARGIN);
    page.set_margin_top(DEFAULT_MARGIN);
    page.set_margin_bottom(DEFAULT_MARGIN);
}


/// Invoke all theme reference refreshers
pub fn refresh_theme_refs(refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>) {
    for refresher in refreshers.borrow().iter() {
        refresher();
    }
}

/// Creates the common theme widgets (colors, gradient, fonts) and appends them to the page.
///
/// This function creates and connects:
/// - 4 color button widgets in a 2x2 grid
/// - 1 gradient editor
/// - 2 font buttons with size spinners
///
/// # Arguments
/// * `page` - The container to append widgets to
/// * `theme` - The current theme config to initialize widgets from
/// * `on_theme_change` - Callback invoked when any theme property changes, receives the updated theme
/// * `on_redraw` - Callback to trigger preview redraw and refresh theme refs
///
/// # Returns
/// A `CommonThemeWidgets` struct containing all created widgets for later reference.
pub fn create_common_theme_widgets<F, R>(
    page: &GtkBox,
    theme: &ComboThemeConfig,
    on_theme_change: F,
    on_redraw: R,
) -> CommonThemeWidgets
where
    F: Fn(Box<dyn FnOnce(&mut ComboThemeConfig)>) + Clone + 'static,
    R: Fn() + Clone + 'static,
{
    // Store current colors for gradient editor sync
    let current_colors = Rc::new(RefCell::new((theme.color1, theme.color2, theme.color3, theme.color4)));

    // Theme Colors section - 2x2 grid layout
    let colors_label = Label::new(Some("Theme Colors"));
    colors_label.set_halign(gtk4::Align::Start);
    colors_label.add_css_class("heading");
    colors_label.set_margin_top(8);
    page.append(&colors_label);

    let colors_grid = gtk4::Grid::new();
    colors_grid.set_row_spacing(6);
    colors_grid.set_column_spacing(8);
    colors_grid.set_margin_start(6);

    // Color 1 (Primary) - row 0, col 0-1
    let color1_label = Label::new(Some("C1 (Primary):"));
    color1_label.set_halign(gtk4::Align::End);
    color1_label.set_width_chars(14);
    colors_grid.attach(&color1_label, 0, 0, 1, 1);
    let color1_widget = Rc::new(ColorButtonWidget::new(theme.color1));
    colors_grid.attach(color1_widget.widget(), 1, 0, 1, 1);

    // Color 2 (Secondary) - row 0, col 2-3
    let color2_label = Label::new(Some("C2 (Secondary):"));
    color2_label.set_halign(gtk4::Align::End);
    color2_label.set_width_chars(14);
    color2_label.set_margin_start(12);
    colors_grid.attach(&color2_label, 2, 0, 1, 1);
    let color2_widget = Rc::new(ColorButtonWidget::new(theme.color2));
    colors_grid.attach(color2_widget.widget(), 3, 0, 1, 1);

    // Color 3 (Accent) - row 1, col 0-1
    let color3_label = Label::new(Some("C3 (Accent):"));
    color3_label.set_halign(gtk4::Align::End);
    color3_label.set_width_chars(14);
    colors_grid.attach(&color3_label, 0, 1, 1, 1);
    let color3_widget = Rc::new(ColorButtonWidget::new(theme.color3));
    colors_grid.attach(color3_widget.widget(), 1, 1, 1, 1);

    // Color 4 (Highlight) - row 1, col 2-3
    let color4_label = Label::new(Some("C4 (Highlight):"));
    color4_label.set_halign(gtk4::Align::End);
    color4_label.set_width_chars(14);
    color4_label.set_margin_start(12);
    colors_grid.attach(&color4_label, 2, 1, 1, 1);
    let color4_widget = Rc::new(ColorButtonWidget::new(theme.color4));
    colors_grid.attach(color4_widget.widget(), 3, 1, 1, 1);

    page.append(&colors_grid);

    // Theme Gradient section (create before color callbacks so we can reference it)
    let gradient_label = Label::new(Some("Theme Gradient"));
    gradient_label.set_halign(gtk4::Align::Start);
    gradient_label.add_css_class("heading");
    gradient_label.set_margin_top(12);
    page.append(&gradient_label);

    let gradient_editor = Rc::new(GradientEditor::new());
    gradient_editor.set_theme_config(theme.clone());
    gradient_editor.set_gradient_source_config(&theme.gradient);
    page.append(gradient_editor.widget());

    // Helper to sync gradient editor's theme colors
    fn sync_gradient_theme(
        gradient_editor: &GradientEditor,
        colors: &Rc<RefCell<(crate::ui::Color, crate::ui::Color, crate::ui::Color, crate::ui::Color)>>,
    ) {
        let (c1, c2, c3, c4) = *colors.borrow();
        let mut theme = ComboThemeConfig::default();
        theme.color1 = c1;
        theme.color2 = c2;
        theme.color3 = c3;
        theme.color4 = c4;
        gradient_editor.set_theme_config(theme);
    }

    // Connect color widget callbacks
    {
        let on_theme_change = on_theme_change.clone();
        let on_redraw = on_redraw.clone();
        let colors = current_colors.clone();
        let ge = gradient_editor.clone();
        color1_widget.set_on_change(move |color| {
            colors.borrow_mut().0 = color;
            on_theme_change(Box::new(move |t| t.color1 = color));
            sync_gradient_theme(&ge, &colors);
            on_redraw();
        });
    }
    {
        let on_theme_change = on_theme_change.clone();
        let on_redraw = on_redraw.clone();
        let colors = current_colors.clone();
        let ge = gradient_editor.clone();
        color2_widget.set_on_change(move |color| {
            colors.borrow_mut().1 = color;
            on_theme_change(Box::new(move |t| t.color2 = color));
            sync_gradient_theme(&ge, &colors);
            on_redraw();
        });
    }
    {
        let on_theme_change = on_theme_change.clone();
        let on_redraw = on_redraw.clone();
        let colors = current_colors.clone();
        let ge = gradient_editor.clone();
        color3_widget.set_on_change(move |color| {
            colors.borrow_mut().2 = color;
            on_theme_change(Box::new(move |t| t.color3 = color));
            sync_gradient_theme(&ge, &colors);
            on_redraw();
        });
    }
    {
        let on_theme_change = on_theme_change.clone();
        let on_redraw = on_redraw.clone();
        let colors = current_colors.clone();
        let ge = gradient_editor.clone();
        color4_widget.set_on_change(move |color| {
            colors.borrow_mut().3 = color;
            on_theme_change(Box::new(move |t| t.color4 = color));
            sync_gradient_theme(&ge, &colors);
            on_redraw();
        });
    }

    {
        let on_theme_change = on_theme_change.clone();
        let on_redraw = on_redraw.clone();
        let gradient_editor_clone = gradient_editor.clone();
        gradient_editor.set_on_change(move || {
            let gradient = gradient_editor_clone.get_gradient_source_config();
            on_theme_change(Box::new(move |t| t.gradient = gradient));
            on_redraw();
        });
    }

    // Theme Fonts section
    let fonts_label = Label::new(Some("Theme Fonts"));
    fonts_label.set_halign(gtk4::Align::Start);
    fonts_label.add_css_class("heading");
    fonts_label.set_margin_top(12);
    page.append(&fonts_label);

    // Font 1
    let font1_box = GtkBox::new(Orientation::Horizontal, 6);
    font1_box.append(&Label::new(Some("Font 1:")));
    let font1_btn = Button::with_label(&theme.font1_family);
    font1_btn.set_hexpand(true);
    font1_box.append(&font1_btn);
    font1_box.append(&Label::new(Some("Size:")));
    let font1_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
    font1_size_spin.set_value(theme.font1_size);
    font1_box.append(&font1_size_spin);
    page.append(&font1_box);

    // Font 1 button click handler
    {
        let on_theme_change = on_theme_change.clone();
        let on_redraw = on_redraw.clone();
        let font1_btn_clone = font1_btn.clone();
        font1_btn.connect_clicked(move |button| {
            let on_theme_change = on_theme_change.clone();
            let on_redraw = on_redraw.clone();
            let font_btn = font1_btn_clone.clone();
            if let Some(window) = button.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) {
                let current_font = font_btn.label().map(|s| s.to_string()).unwrap_or_default();
                let font_desc = gtk4::pango::FontDescription::from_string(&current_font);
                shared_font_dialog().choose_font(
                    Some(&window),
                    Some(&font_desc),
                    gtk4::gio::Cancellable::NONE,
                    move |result| {
                        if let Ok(font_desc) = result {
                            let family = font_desc.family()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "sans-serif".to_string());
                            font_btn.set_label(&family);
                            let family_clone = family.clone();
                            on_theme_change(Box::new(move |t| t.font1_family = family_clone));
                            on_redraw();
                        }
                    },
                );
            }
        });
    }

    // Font 1 size spin handler
    {
        let on_theme_change = on_theme_change.clone();
        let on_redraw = on_redraw.clone();
        font1_size_spin.connect_value_changed(move |spin| {
            let size = spin.value();
            on_theme_change(Box::new(move |t| t.font1_size = size));
            on_redraw();
        });
    }

    // Font 2
    let font2_box = GtkBox::new(Orientation::Horizontal, 6);
    font2_box.append(&Label::new(Some("Font 2:")));
    let font2_btn = Button::with_label(&theme.font2_family);
    font2_btn.set_hexpand(true);
    font2_box.append(&font2_btn);
    font2_box.append(&Label::new(Some("Size:")));
    let font2_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
    font2_size_spin.set_value(theme.font2_size);
    font2_box.append(&font2_size_spin);
    page.append(&font2_box);

    // Font 2 button click handler
    {
        let on_theme_change = on_theme_change.clone();
        let on_redraw = on_redraw.clone();
        let font2_btn_clone = font2_btn.clone();
        font2_btn.connect_clicked(move |button| {
            let on_theme_change = on_theme_change.clone();
            let on_redraw = on_redraw.clone();
            let font_btn = font2_btn_clone.clone();
            if let Some(window) = button.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) {
                let current_font = font_btn.label().map(|s| s.to_string()).unwrap_or_default();
                let font_desc = gtk4::pango::FontDescription::from_string(&current_font);
                shared_font_dialog().choose_font(
                    Some(&window),
                    Some(&font_desc),
                    gtk4::gio::Cancellable::NONE,
                    move |result| {
                        if let Ok(font_desc) = result {
                            let family = font_desc.family()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "sans-serif".to_string());
                            font_btn.set_label(&family);
                            let family_clone = family.clone();
                            on_theme_change(Box::new(move |t| t.font2_family = family_clone));
                            on_redraw();
                        }
                    },
                );
            }
        });
    }

    // Font 2 size spin handler
    {
        let on_theme_change = on_theme_change.clone();
        let on_redraw = on_redraw.clone();
        font2_size_spin.connect_value_changed(move |spin| {
            let size = spin.value();
            on_theme_change(Box::new(move |t| t.font2_size = size));
            on_redraw();
        });
    }

    CommonThemeWidgets {
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

/// Rebuild group weight spinners for any combo panel config that implements LayoutFrameConfig.
///
/// This is a generic function that can be used by all themed config widgets.
pub fn rebuild_group_spinners<C, L, F>(
    container: &GtkBox,
    config: &Rc<RefCell<C>>,
    get_frame: F,
    on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: &DrawingArea,
) where
    C: 'static,
    L: LayoutFrameConfig + ?Sized,
    for<'a> F: Fn(&'a mut C) -> &'a mut L,
    F: Clone + 'static,
{
    // Clear existing children
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let group_count = {
        let cfg = config.borrow();
        let get_frame_clone = get_frame.clone();
        // We need to get group_count without mutable borrow
        // Use a temporary approach - read config, get count
        drop(cfg);
        let mut cfg = config.borrow_mut();
        get_frame_clone(&mut cfg).group_count()
    };

    if group_count <= 1 {
        let label = Label::new(Some("Group weights not applicable for single group."));
        label.add_css_class("dim-label");
        container.append(&label);
        return;
    }

    // Get current weights
    let weights: Vec<f64> = {
        let mut cfg = config.borrow_mut();
        let frame = get_frame(&mut cfg);
        (0..group_count)
            .map(|i| frame.group_size_weights().get(i).copied().unwrap_or(1.0))
            .collect()
    };

    // Create spinners for each group
    for (i, weight) in weights.into_iter().enumerate() {
        let row = GtkBox::new(Orientation::Horizontal, 6);
        row.append(&Label::new(Some(&format!("Group {} Weight:", i + 1))));

        let spin = SpinButton::with_range(0.1, 10.0, 0.1);
        spin.set_digits(1);
        spin.set_value(weight);
        spin.set_hexpand(true);
        row.append(&spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let get_frame_clone = get_frame.clone();
        let idx = i;
        spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            let frame = get_frame_clone(&mut cfg);
            let weights = frame.group_size_weights_mut();
            while weights.len() <= idx {
                weights.push(1.0);
            }
            weights[idx] = spin.value();
            drop(cfg);
            queue_redraw(&preview_clone, &on_change_clone);
        });

        container.append(&row);
    }
}

/// Rebuild item orientation dropdowns for any combo panel config that implements LayoutFrameConfig.
///
/// This is a generic function that can be used by all themed config widgets.
pub fn rebuild_item_orientation_dropdowns<C, L, F>(
    container: &GtkBox,
    config: &Rc<RefCell<C>>,
    get_frame: F,
    on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: &DrawingArea,
) where
    C: 'static,
    L: LayoutFrameConfig + ?Sized,
    for<'a> F: Fn(&'a mut C) -> &'a mut L,
    F: Clone + 'static,
{
    // Clear existing children
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let (group_count, orientations, default_orientation) = {
        let mut cfg = config.borrow_mut();
        let frame = get_frame(&mut cfg);
        let count = frame.group_count();
        let orients: Vec<Option<SplitOrientation>> = (0..count)
            .map(|i| frame.group_item_orientations().get(i).copied())
            .collect();
        let default = frame.split_orientation();
        (count, orients, default)
    };

    if group_count <= 1 {
        let label = Label::new(Some("Item orientation not applicable for single group."));
        label.add_css_class("dim-label");
        container.append(&label);
        return;
    }

    // Create dropdown for each group
    for (group_idx, current) in orientations.into_iter().enumerate() {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        row.append(&Label::new(Some(&format!("Group {}:", group_idx + 1))));

        let options = StringList::new(&["Vertical (stacked)", "Horizontal (side-by-side)", "Default"]);
        let dropdown = DropDown::new(Some(options), gtk4::Expression::NONE);
        dropdown.set_hexpand(true);

        // Determine current selection
        let selected = match current {
            Some(SplitOrientation::Vertical) => 0,
            Some(SplitOrientation::Horizontal) => 1,
            None => 2, // Default
        };
        dropdown.set_selected(selected);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let get_frame_clone = get_frame.clone();
        let default_orient = default_orientation;
        dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            let mut cfg = config_clone.borrow_mut();
            let frame = get_frame_clone(&mut cfg);
            let orientations = frame.group_item_orientations_mut();

            // Ensure the orientations vector is long enough
            while orientations.len() < group_idx {
                orientations.push(default_orient);
            }

            match selected {
                0 => {
                    // Vertical
                    if orientations.len() <= group_idx {
                        orientations.push(SplitOrientation::Vertical);
                    } else {
                        orientations[group_idx] = SplitOrientation::Vertical;
                    }
                }
                1 => {
                    // Horizontal
                    if orientations.len() <= group_idx {
                        orientations.push(SplitOrientation::Horizontal);
                    } else {
                        orientations[group_idx] = SplitOrientation::Horizontal;
                    }
                }
                _ => {
                    // Default - remove explicit orientation if present
                    if orientations.len() > group_idx {
                        orientations.truncate(group_idx);
                    }
                }
            }

            drop(cfg);
            queue_redraw(&preview_clone, &on_change_clone);
        });

        row.append(&dropdown);
        container.append(&row);
    }
}

/// Create the group weights section UI with header and container box.
/// Returns the container box that should be passed to rebuild_group_spinners.
pub fn create_group_weights_section(page: &GtkBox) -> GtkBox {
    let weights_label = Label::new(Some("Group Size Weights"));
    weights_label.set_halign(gtk4::Align::Start);
    weights_label.add_css_class("heading");
    weights_label.set_margin_top(12);
    page.append(&weights_label);

    let group_weights_box = GtkBox::new(Orientation::Vertical, 4);
    page.append(&group_weights_box);

    group_weights_box
}

/// Create the item orientation section UI with header, info label, and container box.
/// Returns the container box that should be passed to rebuild_item_orientation_dropdowns.
pub fn create_item_orientation_section(page: &GtkBox) -> GtkBox {
    let item_orient_label = Label::new(Some("Item Orientation per Group"));
    item_orient_label.set_halign(gtk4::Align::Start);
    item_orient_label.add_css_class("heading");
    item_orient_label.set_margin_top(12);
    page.append(&item_orient_label);

    let item_orient_info = Label::new(Some(
        "Choose how items within each group are arranged",
    ));
    item_orient_info.set_halign(gtk4::Align::Start);
    item_orient_info.add_css_class("dim-label");
    page.append(&item_orient_info);

    let item_orientations_box = GtkBox::new(Orientation::Vertical, 4);
    item_orientations_box.set_margin_top(4);
    page.append(&item_orientations_box);

    item_orientations_box
}

/// Create a theme reference section showing current theme colors, gradient, and fonts
/// with copy buttons for each element.
///
/// Returns the frame and a refresh callback that should be called when theme changes.
pub fn create_theme_reference_section<F, C>(
    config: &Rc<RefCell<C>>,
    get_theme: F,
) -> (gtk4::Frame, Rc<dyn Fn()>)
where
    F: Fn(&C) -> ComboThemeConfig + 'static + Clone,
    C: 'static,
{
    let frame = gtk4::Frame::new(Some("Theme Reference"));
    frame.set_margin_top(8);

    let content_box = GtkBox::new(Orientation::Vertical, 6);
    content_box.set_margin_start(8);
    content_box.set_margin_end(8);
    content_box.set_margin_top(8);
    content_box.set_margin_bottom(8);

    // Colors row
    let colors_box = GtkBox::new(Orientation::Horizontal, 8);
    colors_box.append(&Label::new(Some("Colors:")));

    // Store swatches for refresh
    let color_swatches: Rc<RefCell<Vec<DrawingArea>>> = Rc::new(RefCell::new(Vec::new()));

    let color_indices = [1u8, 2, 3, 4];
    let color_tooltips = [
        "Color 1 (Primary)",
        "Color 2 (Secondary)",
        "Color 3 (Accent)",
        "Color 4 (Highlight)",
    ];

    for (idx, tooltip) in color_indices.iter().zip(color_tooltips.iter()) {
        let item_box = GtkBox::new(Orientation::Horizontal, 2);

        // Color swatch - reads from config dynamically
        let swatch = DrawingArea::new();
        swatch.set_size_request(20, 20);
        let config_for_draw = config.clone();
        let get_theme_for_draw = get_theme.clone();
        let color_idx = *idx;
        swatch.set_draw_func(move |_, cr, width, height| {
            let theme = get_theme_for_draw(&config_for_draw.borrow());
            let c = theme.get_color(color_idx);
            // Draw checkerboard for transparency
            let checker_size = 4.0;
            for y in 0..(height as f64 / checker_size).ceil() as i32 {
                for x in 0..(width as f64 / checker_size).ceil() as i32 {
                    if (x + y) % 2 == 0 {
                        cr.set_source_rgb(0.8, 0.8, 0.8);
                    } else {
                        cr.set_source_rgb(0.6, 0.6, 0.6);
                    }
                    cr.rectangle(
                        x as f64 * checker_size,
                        y as f64 * checker_size,
                        checker_size,
                        checker_size,
                    );
                    let _ = cr.fill();
                }
            }
            // Draw color
            cr.set_source_rgba(c.r, c.g, c.b, c.a);
            cr.rectangle(0.0, 0.0, width as f64, height as f64);
            let _ = cr.fill();
            // Border
            cr.set_source_rgb(0.3, 0.3, 0.3);
            cr.set_line_width(1.0);
            cr.rectangle(0.5, 0.5, width as f64 - 1.0, height as f64 - 1.0);
            let _ = cr.stroke();
        });
        color_swatches.borrow_mut().push(swatch.clone());
        item_box.append(&swatch);

        // Copy button with icon - reads from config dynamically
        let copy_btn = Button::from_icon_name("edit-copy-symbolic");
        copy_btn.set_tooltip_text(Some(&format!("Copy {} to clipboard", tooltip)));
        let config_for_copy = config.clone();
        let get_theme_for_copy = get_theme.clone();
        let color_idx_for_copy = *idx;
        let tooltip_for_log = tooltip.to_string();
        copy_btn.connect_clicked(move |_| {
            let theme = get_theme_for_copy(&config_for_copy.borrow());
            let c = theme.get_color(color_idx_for_copy);
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_color(c.r, c.g, c.b, c.a);
                log::info!("Theme {} copied to clipboard", tooltip_for_log);
            }
        });
        item_box.append(&copy_btn);

        colors_box.append(&item_box);
    }

    // Add separator between colors and gradient
    let separator = gtk4::Separator::new(Orientation::Vertical);
    separator.set_margin_start(4);
    separator.set_margin_end(4);
    colors_box.append(&separator);

    // Gradient preview swatch - in same row as colors
    let gradient_item_box = GtkBox::new(Orientation::Horizontal, 2);
    let gradient_swatch = DrawingArea::new();
    gradient_swatch.set_size_request(50, 20);
    let config_for_gradient = config.clone();
    let get_theme_for_gradient = get_theme.clone();
    gradient_swatch.set_draw_func(move |_, cr, width, height| {
        let theme = get_theme_for_gradient(&config_for_gradient.borrow());
        let gradient_config = theme.gradient.resolve(&theme);
        // Render linear gradient
        let w = width as f64;
        let h = height as f64;
        let angle_rad = gradient_config.angle.to_radians();
        let (dx, dy) = (angle_rad.sin(), -angle_rad.cos());
        let length = (w * dx.abs() + h * dy.abs()) / 2.0;
        let (cx, cy) = (w / 2.0, h / 2.0);
        let (x0, y0) = (cx - dx * length, cy - dy * length);
        let (x1, y1) = (cx + dx * length, cy + dy * length);

        let gradient = gtk4::cairo::LinearGradient::new(x0, y0, x1, y1);
        for stop in &gradient_config.stops {
            gradient.add_color_stop_rgba(
                stop.position,
                stop.color.r,
                stop.color.g,
                stop.color.b,
                stop.color.a,
            );
        }
        let _ = cr.set_source(&gradient);
        cr.rectangle(0.0, 0.0, w, h);
        let _ = cr.fill();

        // Border
        cr.set_source_rgb(0.3, 0.3, 0.3);
        cr.set_line_width(1.0);
        cr.rectangle(0.5, 0.5, w - 1.0, h - 1.0);
        let _ = cr.stroke();
    });
    gradient_item_box.append(&gradient_swatch);

    // Gradient copy button
    let gradient_copy_btn = Button::from_icon_name("edit-copy-symbolic");
    gradient_copy_btn.set_tooltip_text(Some("Copy Theme Gradient to clipboard"));
    let config_for_gradient_copy = config.clone();
    let get_theme_for_gradient_copy = get_theme.clone();
    gradient_copy_btn.connect_clicked(move |_| {
        let theme = get_theme_for_gradient_copy(&config_for_gradient_copy.borrow());
        let resolved_gradient = theme.gradient.resolve(&theme);
        if let Ok(mut clipboard) = CLIPBOARD.lock() {
            clipboard.copy_gradient_stops(resolved_gradient.stops);
            log::info!("Theme gradient copied to clipboard");
        }
    });
    gradient_item_box.append(&gradient_copy_btn);
    colors_box.append(&gradient_item_box);

    content_box.append(&colors_box);

    // Fonts row
    let fonts_box = GtkBox::new(Orientation::Horizontal, 8);
    fonts_box.append(&Label::new(Some("Fonts:")));

    // Store font labels for refresh
    let font_labels: Rc<RefCell<Vec<Label>>> = Rc::new(RefCell::new(Vec::new()));

    let font_indices = [1u8, 2];
    let font_tooltips = ["Font 1 (Headers)", "Font 2 (Content)"];

    for (idx, tooltip) in font_indices.iter().zip(font_tooltips.iter()) {
        let item_box = GtkBox::new(Orientation::Horizontal, 4);

        // Font info label - will be updated on refresh
        let theme = get_theme(&config.borrow());
        let (family, size) = theme.get_font(*idx);
        let info = Label::new(Some(&format!("{} {}pt", family, size as i32)));
        info.add_css_class("dim-label");
        font_labels.borrow_mut().push(info.clone());
        item_box.append(&info);

        // Copy button with icon - copies theme font reference to clipboard
        let copy_btn = Button::from_icon_name("edit-copy-symbolic");
        copy_btn.set_tooltip_text(Some(&format!("Copy {} to clipboard", tooltip)));
        let font_idx = *idx;
        let tooltip_for_log = tooltip.to_string();
        copy_btn.connect_clicked(move |_| {
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_font_source(FontSource::Theme { index: font_idx, size: 14.0 }, false, false);
                log::info!("Theme {} copied to clipboard", tooltip_for_log);
            }
        });
        item_box.append(&copy_btn);

        fonts_box.append(&item_box);
    }

    content_box.append(&fonts_box);
    frame.set_child(Some(&content_box));

    // Create refresh callback
    let config_for_refresh = config.clone();
    let get_theme_for_refresh = get_theme.clone();
    let gradient_swatch_for_refresh = gradient_swatch.clone();
    let refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
        // Refresh color swatches
        for swatch in color_swatches.borrow().iter() {
            swatch.queue_draw();
        }
        // Refresh gradient swatch
        gradient_swatch_for_refresh.queue_draw();
        // Refresh font labels
        let theme = get_theme_for_refresh(&config_for_refresh.borrow());
        let labels = font_labels.borrow();
        if labels.len() >= 2 {
            let (family1, size1) = theme.get_font(1);
            labels[0].set_text(&format!("{} {}pt", family1, size1 as i32));
            let (family2, size2) = theme.get_font(2);
            labels[1].set_text(&format!("{} {}pt", family2, size2 as i32));
        }
    });

    (frame, refresh_callback)
}

/// Create a standard animation page with enable checkbox and speed spinner.
/// Returns the page widget and optionally stores references in the output widgets.
pub fn create_animation_page<C>(
    config: &Rc<RefCell<C>>,
    on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    get_animation_enabled: impl Fn(&C) -> bool + 'static,
    set_animation_enabled: impl Fn(&mut C, bool) + 'static,
    get_animation_speed: impl Fn(&C) -> f64 + 'static,
    set_animation_speed: impl Fn(&mut C, f64) + 'static,
) -> GtkBox
where
    C: 'static,
{
    let page = GtkBox::new(Orientation::Vertical, 8);
    set_page_margins(&page);

    // Enable animation
    let enable_check = CheckButton::with_label("Enable Animations");
    enable_check.set_active(get_animation_enabled(&config.borrow()));

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    enable_check.connect_toggled(move |check| {
        set_animation_enabled(&mut config_clone.borrow_mut(), check.is_active());
        if let Some(cb) = on_change_clone.borrow().as_ref() {
            cb();
        }
    });
    page.append(&enable_check);

    // Animation speed
    let speed_box = GtkBox::new(Orientation::Horizontal, 6);
    speed_box.append(&Label::new(Some("Animation Speed:")));
    let speed_spin = SpinButton::with_range(1.0, 20.0, 1.0);
    speed_spin.set_value(get_animation_speed(&config.borrow()));
    speed_spin.set_hexpand(true);
    speed_box.append(&speed_spin);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    speed_spin.connect_value_changed(move |spin| {
        set_animation_speed(&mut config_clone.borrow_mut(), spin.value());
        if let Some(cb) = on_change_clone.borrow().as_ref() {
            cb();
        }
    });
    page.append(&speed_box);

    page
}

/// Create the content page with tabbed notebook for content items.
pub fn create_content_page<C, F, S, G>(
    config: &Rc<RefCell<C>>,
    on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: &DrawingArea,
    content_notebook: &Rc<RefCell<Notebook>>,
    source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
    available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
    get_content_items: F,
    set_content_item: S,
    theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    get_theme: G,
) -> GtkBox
where
    C: 'static,
    F: Fn(&C) -> &HashMap<String, ContentItemConfig> + Clone + 'static,
    S: Fn(&mut C, &str, ContentItemConfig) + Clone + 'static,
    G: Fn(&C) -> ComboThemeConfig + Clone + 'static,
{
    let page = GtkBox::new(Orientation::Vertical, 8);
    set_page_margins(&page);

    let info_label = Label::new(Some(
        "Content items are configured per source slot.\nSelect a slot tab to configure its display type.",
    ));
    info_label.set_halign(gtk4::Align::Start);
    info_label.add_css_class("dim-label");
    page.append(&info_label);

    // Create scrolled window for content tabs
    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    {
        let notebook = content_notebook.borrow();
        scroll.set_child(Some(&*notebook));
    }

    page.append(&scroll);

    // Build initial content tabs
    rebuild_content_tabs(
        config,
        on_change,
        preview,
        content_notebook,
        source_summaries,
        available_fields,
        get_content_items,
        set_content_item,
        theme_ref_refreshers,
        get_theme,
    );

    page
}

/// Generation counter for canceling stale content tab rebuilds
static CONTENT_REBUILD_GENERATION: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Rebuild the content tabs based on source summaries.
/// This function builds tabs incrementally to avoid freezing the UI.
pub fn rebuild_content_tabs<C, F, S, G>(
    config: &Rc<RefCell<C>>,
    on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: &DrawingArea,
    content_notebook: &Rc<RefCell<Notebook>>,
    source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
    available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
    get_content_items: F,
    set_content_item: S,
    theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    get_theme: G,
) where
    C: 'static,
    F: Fn(&C) -> &HashMap<String, ContentItemConfig> + Clone + 'static,
    S: Fn(&mut C, &str, ContentItemConfig) + Clone + 'static,
    G: Fn(&C) -> ComboThemeConfig + Clone + 'static,
{
    // Increment generation to cancel any pending incremental builds
    let generation = CONTENT_REBUILD_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;

    // CRITICAL: Clear stale theme refresh callbacks before rebuilding tabs.
    // Each content item adds multiple callbacks, and without clearing,
    // these accumulate on every rebuild causing memory leaks and CPU explosion.
    theme_ref_refreshers.borrow_mut().clear();

    let notebook = content_notebook.borrow();

    // Clear existing tabs
    while notebook.n_pages() > 0 {
        notebook.remove_page(Some(0));
    }

    let summaries = source_summaries.borrow();

    if summaries.is_empty() {
        let placeholder = GtkBox::new(Orientation::Vertical, 8);
        placeholder.set_margin_start(12);
        placeholder.set_margin_end(12);
        placeholder.set_margin_top(12);
        let label = Label::new(Some(
            "No sources configured.\nGo to 'Data Source' tab and select 'Combination' source to configure content.",
        ));
        label.set_halign(gtk4::Align::Start);
        placeholder.append(&label);
        notebook.append_page(&placeholder, Some(&Label::new(Some("No Sources"))));
        return;
    }

    // Group summaries by group number
    let mut groups: HashMap<usize, Vec<(String, String, u32)>> = HashMap::new();
    for (slot_name, summary, group_num, item_idx) in summaries.iter() {
        groups
            .entry(*group_num)
            .or_default()
            .push((slot_name.clone(), summary.clone(), *item_idx));
    }

    let mut group_nums: Vec<usize> = groups.keys().cloned().collect();
    group_nums.sort();

    // Collect all items to create, along with their group structure
    // Format: Vec<(group_num, items_notebook, Vec<(slot_name, tab_label)>)>
    let mut work_items: Vec<(usize, Notebook, GtkBox, Vec<(String, String)>)> = Vec::new();

    for group_num in group_nums {
        if let Some(items) = groups.get(&group_num) {
            let group_box = GtkBox::new(Orientation::Vertical, 4);
            group_box.set_margin_start(4);
            group_box.set_margin_end(4);
            group_box.set_margin_top(4);

            let items_notebook = Notebook::new();
            items_notebook.set_scrollable(true);
            items_notebook.set_vexpand(true);

            let mut sorted_items = items.clone();
            sorted_items.sort_by_key(|(_, _, idx)| *idx);

            let item_list: Vec<(String, String)> = sorted_items
                .iter()
                .map(|(slot_name, summary, item_idx)| {
                    (slot_name.clone(), format!("Item {} : {}", item_idx, summary))
                })
                .collect();

            // Add placeholder tabs immediately (cheap operation)
            for (_, tab_label) in &item_list {
                let placeholder = GtkBox::new(Orientation::Vertical, 8);
                placeholder.set_margin_top(12);
                placeholder.set_halign(gtk4::Align::Center);
                placeholder.set_valign(gtk4::Align::Center);
                let spinner = gtk4::Spinner::new();
                spinner.start();
                placeholder.append(&spinner);
                let label = Label::new(Some("Loading..."));
                label.add_css_class("dim-label");
                placeholder.append(&label);
                items_notebook.append_page(&placeholder, Some(&Label::new(Some(tab_label))));
            }

            group_box.append(&items_notebook);
            notebook.append_page(
                &group_box,
                Some(&Label::new(Some(&format!("Group {}", group_num)))),
            );

            work_items.push((group_num, items_notebook, group_box, item_list));
        }
    }

    // Release borrow before starting async work
    drop(summaries);
    drop(notebook);

    // Build content items incrementally using idle callbacks
    let work_queue: Rc<RefCell<Vec<(Notebook, usize, String, String)>>> = Rc::new(RefCell::new(Vec::new()));

    // Flatten work items into a queue of individual items to create
    for (_group_num, items_notebook, _group_box, item_list) in work_items {
        for (idx, (slot_name, tab_label)) in item_list.into_iter().enumerate() {
            work_queue.borrow_mut().push((items_notebook.clone(), idx, slot_name, tab_label));
        }
    }

    // If no work to do, return early
    if work_queue.borrow().is_empty() {
        return;
    }

    // Start incremental building
    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    let preview_clone = preview.clone();
    let available_fields_clone = available_fields.clone();
    let theme_ref_refreshers_clone = theme_ref_refreshers.clone();

    glib::idle_add_local(move || {
        // Check if this build has been superseded
        if CONTENT_REBUILD_GENERATION.load(std::sync::atomic::Ordering::SeqCst) != generation {
            return glib::ControlFlow::Break;
        }

        // Get next item to create
        let next_item = work_queue.borrow_mut().pop();

        if let Some((items_notebook, page_idx, slot_name, _tab_label)) = next_item {
            // Create the actual content item config
            let tab_box = create_content_item_config(
                &config_clone,
                &on_change_clone,
                &preview_clone,
                &slot_name,
                available_fields_clone.borrow().clone(),
                get_content_items.clone(),
                set_content_item.clone(),
                &theme_ref_refreshers_clone,
                get_theme.clone(),
            );

            // Replace the placeholder with the actual content
            // Get the current page at this index and remove it, then insert new one
            if let Some(page) = items_notebook.nth_page(Some(page_idx as u32)) {
                let tab_label_widget = items_notebook.tab_label(&page);
                items_notebook.remove_page(Some(page_idx as u32));
                items_notebook.insert_page(&tab_box, tab_label_widget.as_ref(), Some(page_idx as u32));
            }

            glib::ControlFlow::Continue
        } else {
            // All items created
            glib::ControlFlow::Break
        }
    });
}

/// Create configuration UI for a single content item.
pub fn create_content_item_config<C, F, S, G>(
    config: &Rc<RefCell<C>>,
    on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: &DrawingArea,
    slot_name: &str,
    available_fields: Vec<FieldMetadata>,
    get_content_items: F,
    set_content_item: S,
    theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    get_theme: G,
) -> GtkBox
where
    C: 'static,
    F: Fn(&C) -> &HashMap<String, ContentItemConfig> + Clone + 'static,
    S: Fn(&mut C, &str, ContentItemConfig) + Clone + 'static,
    G: Fn(&C) -> ComboThemeConfig + Clone + 'static,
{
    // Need a way to get mutable access to content_items
    // For now we'll use a trait object approach
    let tab = create_page_container();

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let inner_box = GtkBox::new(Orientation::Vertical, 8);

    // Display type dropdown
    let type_box = GtkBox::new(Orientation::Horizontal, 6);
    type_box.append(&Label::new(Some("Display As:")));
    let type_list = StringList::new(&[
        "Bar",
        "Text",
        "Graph",
        "Core Bars",
        "Static",
        "Arc",
        "Speedometer",
    ]);
    let type_dropdown = DropDown::new(Some(type_list), None::<gtk4::Expression>);
    type_dropdown.set_hexpand(true);

    let current_type = {
        let cfg = config.borrow();
        get_content_items(&cfg)
            .get(slot_name)
            .map(|item| item.display_as)
            .unwrap_or_default()
    };
    let type_idx = match current_type {
        ContentDisplayType::Bar | ContentDisplayType::LevelBar => 0,
        ContentDisplayType::Text => 1,
        ContentDisplayType::Graph => 2,
        ContentDisplayType::CoreBars => 3,
        ContentDisplayType::Static => 4,
        ContentDisplayType::Arc => 5,
        ContentDisplayType::Speedometer => 6,
    };
    type_dropdown.set_selected(type_idx);
    type_box.append(&type_dropdown);
    inner_box.append(&type_box);

    // Auto height checkbox
    let auto_height_check = CheckButton::with_label("Auto-adjust height");
    let current_auto_height = {
        let cfg = config.borrow();
        get_content_items(&cfg)
            .get(slot_name)
            .map(|item| item.auto_height)
            .unwrap_or(true)
    };
    auto_height_check.set_active(current_auto_height);
    inner_box.append(&auto_height_check);

    // Item height
    let height_box = GtkBox::new(Orientation::Horizontal, 6);
    height_box.append(&Label::new(Some("Item Height:")));
    let height_spin = SpinButton::with_range(20.0, 300.0, 5.0);
    let current_height = {
        let cfg = config.borrow();
        get_content_items(&cfg)
            .get(slot_name)
            .map(|item| item.item_height)
            .unwrap_or(60.0)
    };
    height_spin.set_value(current_height);
    height_spin.set_hexpand(true);
    height_spin.set_sensitive(!current_auto_height);
    height_box.append(&height_spin);
    inner_box.append(&height_box);

    // Connect auto-height checkbox
    {
        let height_spin_clone = height_spin.clone();
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let get_content_items_clone = get_content_items.clone();
        let set_content_item_clone = set_content_item.clone();
        auto_height_check.connect_toggled(move |check| {
            let is_auto = check.is_active();
            height_spin_clone.set_sensitive(!is_auto);
            let mut cfg = config_clone.borrow_mut();
            let mut item = get_content_items_clone(&cfg)
                .get(&slot_name_clone)
                .cloned()
                .unwrap_or_default();
            item.auto_height = is_auto;
            set_content_item_clone(&mut cfg, &slot_name_clone, item);
            drop(cfg);
            queue_redraw(&preview_clone, &on_change_clone);
        });
    }

    // Connect height spinner
    {
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let get_content_items_clone = get_content_items.clone();
        let set_content_item_clone = set_content_item.clone();
        height_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            let mut item = get_content_items_clone(&cfg)
                .get(&slot_name_clone)
                .cloned()
                .unwrap_or_default();
            item.item_height = spin.value();
            set_content_item_clone(&mut cfg, &slot_name_clone, item);
            drop(cfg);
            queue_redraw(&preview_clone, &on_change_clone);
        });
    }

    // Get available fields for this slot
    let slot_prefix = format!("{}_", slot_name);
    let mut slot_fields: Vec<FieldMetadata> = available_fields
        .iter()
        .filter(|f| f.id.starts_with(&slot_prefix))
        .map(|f| {
            let short_id = f.id.strip_prefix(&slot_prefix).unwrap_or(&f.id);
            FieldMetadata::new(
                short_id,
                &f.name,
                &f.description,
                f.field_type.clone(),
                f.purpose.clone(),
            )
        })
        .collect();

    if slot_fields.is_empty() {
        slot_fields = vec![
            FieldMetadata::new(
                "caption",
                "Caption",
                "Label text",
                FieldType::Text,
                FieldPurpose::Caption,
            ),
            FieldMetadata::new(
                "value",
                "Value",
                "Current value",
                FieldType::Text,
                FieldPurpose::Value,
            ),
            FieldMetadata::new(
                "unit",
                "Unit",
                "Unit of measurement",
                FieldType::Text,
                FieldPurpose::Unit,
            ),
            FieldMetadata::new(
                "numerical_value",
                "Numeric Value",
                "Raw numeric value",
                FieldType::Numerical,
                FieldPurpose::Value,
            ),
        ];
    }

    // === Bar Configuration Section (Lazy-loaded for performance) ===
    let bar_config_frame = gtk4::Frame::new(Some("Bar Configuration"));
    bar_config_frame.set_margin_top(12);

    // Use LazyBarConfigWidget to defer expensive widget creation until user clicks
    let bar_widget = LazyBarConfigWidget::new(slot_fields.clone());
    // Set theme BEFORE config, since set_config triggers UI rebuild that needs theme
    {
        let cfg = config.borrow();
        bar_widget.set_theme(get_theme(&cfg));
    }
    let current_bar_config = {
        let cfg = config.borrow();
        get_content_items(&cfg)
            .get(slot_name)
            .map(|item| item.bar_config.clone())
            .unwrap_or_default()
    };
    bar_widget.set_config(current_bar_config);

    // Connect bar widget on_change
    let bar_widget_rc = Rc::new(bar_widget);
    {
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let slot_name_clone = slot_name.to_string();
        let get_content_items_clone = get_content_items.clone();
        let set_content_item_clone = set_content_item.clone();
        let bar_widget_for_cb = bar_widget_rc.clone();
        bar_widget_rc.set_on_change(move || {
            let bar_config = bar_widget_for_cb.get_config();
            let mut cfg = config_clone.borrow_mut();
            let mut item = get_content_items_clone(&cfg)
                .get(&slot_name_clone)
                .cloned()
                .unwrap_or_default();
            item.bar_config = bar_config;
            set_content_item_clone(&mut cfg, &slot_name_clone, item);
            drop(cfg);
            queue_redraw(&preview_clone, &on_change_clone);
        });
    }

    // Register theme refresh callback for bar widget
    {
        let bar_widget_for_theme = bar_widget_rc.clone();
        let config_for_bar_theme = config.clone();
        let get_theme_for_bar = get_theme.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = get_theme_for_bar(&config_for_bar_theme.borrow());
            bar_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);
    }

    bar_config_frame.set_child(Some(bar_widget_rc.widget()));
    inner_box.append(&bar_config_frame);

    // === Graph Configuration Section (Lazy-loaded for performance) ===
    let graph_config_frame = gtk4::Frame::new(Some("Graph Configuration"));
    graph_config_frame.set_margin_top(12);

    // Use LazyGraphConfigWidget to defer expensive widget creation until user clicks
    let graph_widget = LazyGraphConfigWidget::new(slot_fields.clone());
    // Set theme BEFORE config, since set_config triggers UI rebuild that needs theme
    {
        let cfg = config.borrow();
        graph_widget.set_theme(get_theme(&cfg));
    }
    let current_graph_config = {
        let cfg = config.borrow();
        get_content_items(&cfg)
            .get(slot_name)
            .map(|item| item.graph_config.clone())
            .unwrap_or_default()
    };
    graph_widget.set_config(current_graph_config);

    // Connect graph widget on_change
    let graph_widget_rc = Rc::new(graph_widget);
    {
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let slot_name_clone = slot_name.to_string();
        let get_content_items_clone = get_content_items.clone();
        let set_content_item_clone = set_content_item.clone();
        let graph_widget_for_cb = graph_widget_rc.clone();
        graph_widget_rc.set_on_change(move || {
            let graph_config = graph_widget_for_cb.get_config();
            let mut cfg = config_clone.borrow_mut();
            let mut item = get_content_items_clone(&cfg)
                .get(&slot_name_clone)
                .cloned()
                .unwrap_or_default();
            item.graph_config = graph_config;
            set_content_item_clone(&mut cfg, &slot_name_clone, item);
            drop(cfg);
            queue_redraw(&preview_clone, &on_change_clone);
        });
    }

    // Register theme refresh callback for graph widget
    {
        let graph_widget_for_theme = graph_widget_rc.clone();
        let config_for_graph_theme = config.clone();
        let get_theme_for_graph = get_theme.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = get_theme_for_graph(&config_for_graph_theme.borrow());
            graph_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);
    }

    graph_config_frame.set_child(Some(graph_widget_rc.widget()));
    inner_box.append(&graph_config_frame);

    // === Text Overlay Section (Lazy-loaded for performance) ===
    let text_config_frame = gtk4::Frame::new(Some("Text Overlay"));
    text_config_frame.set_margin_top(12);

    // Use LazyTextOverlayConfigWidget to defer expensive widget creation until user clicks
    let text_widget = LazyTextOverlayConfigWidget::new(slot_fields.clone());
    // Set theme BEFORE config, since set_config triggers UI rebuild that needs theme
    {
        let cfg = config.borrow();
        text_widget.set_theme(get_theme(&cfg));
    }
    let current_text_overlay = {
        let cfg = config.borrow();
        get_content_items(&cfg)
            .get(slot_name)
            .map(|item| item.bar_config.text_overlay.clone())
            .unwrap_or_default()
    };
    text_widget.set_config(current_text_overlay);

    // Connect text widget on_change
    let text_widget_rc = Rc::new(text_widget);
    {
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let slot_name_clone = slot_name.to_string();
        let get_content_items_clone = get_content_items.clone();
        // Only save when Text display mode is active to avoid overwriting BarConfigWidget's changes
        let set_content_item_clone = set_content_item.clone();
        let text_widget_for_cb = text_widget_rc.clone();
        text_widget_rc.set_on_change(move || {
            let mut cfg = config_clone.borrow_mut();
            let mut item = get_content_items_clone(&cfg)
                .get(&slot_name_clone)
                .cloned()
                .unwrap_or_default();
            // Only update if Text mode is active (not Bar mode which has its own text widget)
            let is_text_mode = matches!(item.display_as, ContentDisplayType::Text | ContentDisplayType::Static);
            if is_text_mode {
                item.bar_config.text_overlay = text_widget_for_cb.get_config();
            }
            set_content_item_clone(&mut cfg, &slot_name_clone, item);
            drop(cfg);
            queue_redraw(&preview_clone, &on_change_clone);
        });
    }

    // Register theme refresh callback for text widget
    {
        let text_widget_for_theme = text_widget_rc.clone();
        let config_for_text_theme = config.clone();
        let get_theme_for_text = get_theme.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = get_theme_for_text(&config_for_text_theme.borrow());
            text_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);
    }

    text_config_frame.set_child(Some(text_widget_rc.widget()));
    inner_box.append(&text_config_frame);

    // === Core Bars Configuration Section ===
    let core_bars_config_frame = gtk4::Frame::new(Some("Core Bars Configuration"));
    core_bars_config_frame.set_margin_top(12);

    let core_bars_widget = CoreBarsConfigWidget::new();
    let current_core_bars_config = {
        let cfg = config.borrow();
        get_content_items(&cfg)
            .get(slot_name)
            .map(|item| item.core_bars_config.clone())
            .unwrap_or_default()
    };
    // Set theme BEFORE config, since set_config triggers UI rebuild that needs theme
    {
        let cfg = config.borrow();
        core_bars_widget.set_theme(get_theme(&cfg));
    }
    core_bars_widget.set_config(current_core_bars_config);

    // Connect core bars widget on_change
    let core_bars_widget_rc = Rc::new(core_bars_widget);
    {
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let slot_name_clone = slot_name.to_string();
        let get_content_items_clone = get_content_items.clone();
        let set_content_item_clone = set_content_item.clone();
        let core_bars_widget_for_cb = core_bars_widget_rc.clone();
        core_bars_widget_rc.set_on_change(move || {
            let core_bars_config = core_bars_widget_for_cb.get_config();
            let mut cfg = config_clone.borrow_mut();
            let mut item = get_content_items_clone(&cfg)
                .get(&slot_name_clone)
                .cloned()
                .unwrap_or_default();
            item.core_bars_config = core_bars_config;
            set_content_item_clone(&mut cfg, &slot_name_clone, item);
            drop(cfg);
            queue_redraw(&preview_clone, &on_change_clone);
        });
    }

    // Register theme refresh callback for core bars widget
    {
        let core_bars_widget_for_theme = core_bars_widget_rc.clone();
        let config_for_core_bars_theme = config.clone();
        let get_theme_for_core_bars = get_theme.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = get_theme_for_core_bars(&config_for_core_bars_theme.borrow());
            core_bars_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);
    }

    core_bars_config_frame.set_child(Some(core_bars_widget_rc.widget()));
    inner_box.append(&core_bars_config_frame);

    // === Arc Configuration Section ===
    let arc_config_frame = gtk4::Frame::new(Some("Arc Configuration"));
    arc_config_frame.set_margin_top(12);

    let arc_widget = ArcConfigWidget::new(slot_fields.clone());
    // Set theme BEFORE config, since set_config triggers UI rebuild that needs theme
    {
        let cfg = config.borrow();
        arc_widget.set_theme(get_theme(&cfg));
    }
    let current_arc_config = {
        let cfg = config.borrow();
        get_content_items(&cfg)
            .get(slot_name)
            .map(|item| item.arc_config.clone())
            .unwrap_or_default()
    };
    arc_widget.set_config(current_arc_config);

    // Connect arc widget on_change
    let arc_widget_rc = Rc::new(arc_widget);
    {
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let slot_name_clone = slot_name.to_string();
        let get_content_items_clone = get_content_items.clone();
        let set_content_item_clone = set_content_item.clone();
        let arc_widget_for_cb = arc_widget_rc.clone();
        arc_widget_rc.set_on_change(move || {
            let arc_config = arc_widget_for_cb.get_config();
            let mut cfg = config_clone.borrow_mut();
            let mut item = get_content_items_clone(&cfg)
                .get(&slot_name_clone)
                .cloned()
                .unwrap_or_default();
            item.arc_config = arc_config;
            set_content_item_clone(&mut cfg, &slot_name_clone, item);
            drop(cfg);
            queue_redraw(&preview_clone, &on_change_clone);
        });
    }

    // Register theme refresh callback for arc widget
    {
        let arc_widget_for_theme = arc_widget_rc.clone();
        let config_for_arc_theme = config.clone();
        let get_theme_for_arc = get_theme.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = get_theme_for_arc(&config_for_arc_theme.borrow());
            arc_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);
    }

    arc_config_frame.set_child(Some(arc_widget_rc.widget()));
    inner_box.append(&arc_config_frame);

    // === Speedometer Configuration Section ===
    let speedometer_config_frame = gtk4::Frame::new(Some("Speedometer Configuration"));
    speedometer_config_frame.set_margin_top(12);

    let speedometer_widget = SpeedometerConfigWidget::new(slot_fields.clone());
    // Set theme BEFORE config, since set_config triggers UI rebuild that needs theme
    {
        let cfg = config.borrow();
        speedometer_widget.set_theme(get_theme(&cfg));
    }
    let current_speedometer_config = {
        let cfg = config.borrow();
        get_content_items(&cfg)
            .get(slot_name)
            .map(|item| item.speedometer_config.clone())
            .unwrap_or_default()
    };
    speedometer_widget.set_config(&current_speedometer_config);

    // Connect speedometer widget on_change
    let speedometer_widget_rc = Rc::new(speedometer_widget);
    {
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let slot_name_clone = slot_name.to_string();
        let get_content_items_clone = get_content_items.clone();
        let set_content_item_clone = set_content_item.clone();
        let speedometer_widget_for_cb = speedometer_widget_rc.clone();
        speedometer_widget_rc.set_on_change(Box::new(move || {
            let speedometer_config = speedometer_widget_for_cb.get_config();
            let mut cfg = config_clone.borrow_mut();
            let mut item = get_content_items_clone(&cfg)
                .get(&slot_name_clone)
                .cloned()
                .unwrap_or_default();
            item.speedometer_config = speedometer_config;
            set_content_item_clone(&mut cfg, &slot_name_clone, item);
            drop(cfg);
            queue_redraw(&preview_clone, &on_change_clone);
        }));
    }

    // Register theme refresh callback for speedometer widget
    {
        let speedometer_widget_for_theme = speedometer_widget_rc.clone();
        let config_for_speedometer_theme = config.clone();
        let get_theme_for_speedometer = get_theme.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = get_theme_for_speedometer(&config_for_speedometer_theme.borrow());
            speedometer_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);
    }

    speedometer_config_frame.set_child(Some(speedometer_widget_rc.widget()));
    inner_box.append(&speedometer_config_frame);

    // === Static Display Configuration Section ===
    let static_config_frame = gtk4::Frame::new(Some("Static Display Configuration"));
    static_config_frame.set_margin_top(12);

    let static_widget = StaticConfigWidget::new(slot_fields.clone());
    // Set theme BEFORE config, since set_config triggers UI rebuild that needs theme
    {
        let cfg = config.borrow();
        static_widget.set_theme(get_theme(&cfg));
    }
    let current_static_config = {
        let cfg = config.borrow();
        get_content_items(&cfg)
            .get(slot_name)
            .map(|item| item.static_config.clone())
            .unwrap_or_default()
    };
    static_widget.set_config(current_static_config);

    // Connect static widget on_change
    let static_widget_rc = Rc::new(static_widget);
    {
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let slot_name_clone = slot_name.to_string();
        let get_content_items_clone = get_content_items.clone();
        let set_content_item_clone = set_content_item.clone();
        let static_widget_for_cb = static_widget_rc.clone();
        static_widget_rc.set_on_change(move || {
            let static_config = static_widget_for_cb.get_config();
            let mut cfg = config_clone.borrow_mut();
            let mut item = get_content_items_clone(&cfg)
                .get(&slot_name_clone)
                .cloned()
                .unwrap_or_default();
            item.static_config = static_config;
            set_content_item_clone(&mut cfg, &slot_name_clone, item);
            drop(cfg);
            queue_redraw(&preview_clone, &on_change_clone);
        });
    }

    // Register theme refresh callback for static widget
    {
        let static_widget_for_theme = static_widget_rc.clone();
        let config_for_static_theme = config.clone();
        let get_theme_for_static = get_theme.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = get_theme_for_static(&config_for_static_theme.borrow());
            static_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);
    }

    static_config_frame.set_child(Some(static_widget_rc.widget()));
    inner_box.append(&static_config_frame);

    // Show/hide frames based on display type
    let show_frame_for_type = |display_type: ContentDisplayType| {
        bar_config_frame.set_visible(matches!(
            display_type,
            ContentDisplayType::Bar | ContentDisplayType::LevelBar
        ));
        graph_config_frame.set_visible(matches!(display_type, ContentDisplayType::Graph));
        text_config_frame.set_visible(matches!(display_type, ContentDisplayType::Text));
        core_bars_config_frame.set_visible(matches!(display_type, ContentDisplayType::CoreBars));
        arc_config_frame.set_visible(matches!(display_type, ContentDisplayType::Arc));
        speedometer_config_frame
            .set_visible(matches!(display_type, ContentDisplayType::Speedometer));
        static_config_frame.set_visible(matches!(display_type, ContentDisplayType::Static));
    };

    // Set initial visibility
    show_frame_for_type(current_type);

    // Connect display type dropdown
    let bar_config_frame_clone = bar_config_frame.clone();
    let graph_config_frame_clone = graph_config_frame.clone();
    let text_config_frame_clone = text_config_frame.clone();
    let core_bars_config_frame_clone = core_bars_config_frame.clone();
    let arc_config_frame_clone = arc_config_frame.clone();
    let speedometer_config_frame_clone = speedometer_config_frame.clone();
    let static_config_frame_clone = static_config_frame.clone();
    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    let preview_clone = preview.clone();
    let slot_name_for_dropdown = slot_name.to_string();
    let get_content_items_clone = get_content_items.clone();
    let set_content_item_clone = set_content_item.clone();
    type_dropdown.connect_selected_notify(move |dropdown| {
        let selected = dropdown.selected();
        if selected == gtk4::INVALID_LIST_POSITION {
            return;
        }
        let display_type = match selected {
            0 => ContentDisplayType::Bar,
            1 => ContentDisplayType::Text,
            2 => ContentDisplayType::Graph,
            3 => ContentDisplayType::CoreBars,
            4 => ContentDisplayType::Static,
            5 => ContentDisplayType::Arc,
            6 => ContentDisplayType::Speedometer,
            _ => ContentDisplayType::Bar,
        };

        // Update config
        {
            let mut cfg = config_clone.borrow_mut();
            // Get current item or create default
            let current_item = get_content_items_clone(&cfg)
                .get(&slot_name_for_dropdown)
                .cloned()
                .unwrap_or_default();
            let mut new_item = current_item;
            new_item.display_as = display_type;
            set_content_item_clone(&mut cfg, &slot_name_for_dropdown, new_item);
        }

        // Update frame visibility
        bar_config_frame_clone.set_visible(matches!(
            display_type,
            ContentDisplayType::Bar | ContentDisplayType::LevelBar
        ));
        graph_config_frame_clone.set_visible(matches!(display_type, ContentDisplayType::Graph));
        text_config_frame_clone.set_visible(matches!(display_type, ContentDisplayType::Text));
        core_bars_config_frame_clone
            .set_visible(matches!(display_type, ContentDisplayType::CoreBars));
        arc_config_frame_clone.set_visible(matches!(display_type, ContentDisplayType::Arc));
        speedometer_config_frame_clone
            .set_visible(matches!(display_type, ContentDisplayType::Speedometer));
        static_config_frame_clone.set_visible(matches!(display_type, ContentDisplayType::Static));

        // Trigger redraw and notify change
        queue_redraw(&preview_clone, &on_change_clone);
    });

    scroll.set_child(Some(&inner_box));
    tab.append(&scroll);

    tab
}

/// Create a layout page with common settings (split orientation, padding, spacing, dividers).
/// The divider styles are panel-specific and must be provided.
pub fn create_layout_page_common<C>(
    config: &Rc<RefCell<C>>,
    on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: &DrawingArea,
    get_split_orientation: impl Fn(&C) -> SplitOrientation + 'static,
    set_split_orientation: impl Fn(&mut C, SplitOrientation) + 'static,
    get_content_padding: impl Fn(&C) -> f64 + 'static,
    set_content_padding: impl Fn(&mut C, f64) + 'static,
    get_item_spacing: impl Fn(&C) -> f64 + 'static,
    set_item_spacing: impl Fn(&mut C, f64) + 'static,
    get_divider_padding: impl Fn(&C) -> f64 + 'static,
    set_divider_padding: impl Fn(&mut C, f64) + 'static,
) -> GtkBox
where
    C: 'static,
{
    let page = GtkBox::new(Orientation::Vertical, 8);
    set_page_margins(&page);

    // Split orientation
    let orient_box = GtkBox::new(Orientation::Horizontal, 6);
    orient_box.append(&Label::new(Some("Split Orientation:")));
    let orient_list = StringList::new(&["Vertical", "Horizontal"]);
    let split_orientation_dropdown = DropDown::new(Some(orient_list), None::<gtk4::Expression>);
    let orient_idx = match get_split_orientation(&config.borrow()) {
        SplitOrientation::Vertical => 0,
        SplitOrientation::Horizontal => 1,
    };
    split_orientation_dropdown.set_selected(orient_idx);
    split_orientation_dropdown.set_hexpand(true);
    orient_box.append(&split_orientation_dropdown);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    let preview_clone = preview.clone();
    split_orientation_dropdown.connect_selected_notify(move |dropdown| {
        let selected = dropdown.selected();
        if selected == gtk4::INVALID_LIST_POSITION {
            return;
        }
        let orientation = match selected {
            0 => SplitOrientation::Vertical,
            _ => SplitOrientation::Horizontal,
        };
        set_split_orientation(&mut config_clone.borrow_mut(), orientation);
        queue_redraw(&preview_clone, &on_change_clone);
    });
    page.append(&orient_box);

    // Content padding
    let padding_box = GtkBox::new(Orientation::Horizontal, 6);
    padding_box.append(&Label::new(Some("Content Padding:")));
    let content_padding_spin = SpinButton::with_range(4.0, 32.0, 2.0);
    content_padding_spin.set_value(get_content_padding(&config.borrow()));
    content_padding_spin.set_hexpand(true);
    padding_box.append(&content_padding_spin);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    let preview_clone = preview.clone();
    content_padding_spin.connect_value_changed(move |spin| {
        set_content_padding(&mut config_clone.borrow_mut(), spin.value());
        queue_redraw(&preview_clone, &on_change_clone);
    });
    page.append(&padding_box);

    // Item spacing
    let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
    spacing_box.append(&Label::new(Some("Item Spacing:")));
    let item_spacing_spin = SpinButton::with_range(0.0, 20.0, 1.0);
    item_spacing_spin.set_value(get_item_spacing(&config.borrow()));
    item_spacing_spin.set_hexpand(true);
    spacing_box.append(&item_spacing_spin);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    let preview_clone = preview.clone();
    item_spacing_spin.connect_value_changed(move |spin| {
        set_item_spacing(&mut config_clone.borrow_mut(), spin.value());
        queue_redraw(&preview_clone, &on_change_clone);
    });
    page.append(&spacing_box);

    // Divider section header
    let divider_label = Label::new(Some("Dividers"));
    divider_label.set_halign(gtk4::Align::Start);
    divider_label.add_css_class("heading");
    divider_label.set_margin_top(12);
    page.append(&divider_label);

    // Divider padding
    let div_padding_box = GtkBox::new(Orientation::Horizontal, 6);
    div_padding_box.append(&Label::new(Some("Divider Padding:")));
    let divider_padding_spin = SpinButton::with_range(2.0, 20.0, 1.0);
    divider_padding_spin.set_value(get_divider_padding(&config.borrow()));
    divider_padding_spin.set_hexpand(true);
    div_padding_box.append(&divider_padding_spin);

    let config_clone = config.clone();
    let on_change_clone = on_change.clone();
    let preview_clone = preview.clone();
    divider_padding_spin.connect_value_changed(move |spin| {
        set_divider_padding(&mut config_clone.borrow_mut(), spin.value());
        queue_redraw(&preview_clone, &on_change_clone);
    });
    page.append(&div_padding_box);

    page
}

/// Create a theme page with common color and font controls.
/// Panel-specific widgets should be added after calling this.
pub fn create_theme_page_base<C>(
    config: &Rc<RefCell<C>>,
    on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: &DrawingArea,
    theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    get_theme: impl Fn(&C) -> ComboThemeConfig + Clone + 'static,
    set_color1: impl Fn(&mut C, crate::ui::background::Color) + 'static,
    set_color2: impl Fn(&mut C, crate::ui::background::Color) + 'static,
    set_color3: impl Fn(&mut C, crate::ui::background::Color) + 'static,
    set_color4: impl Fn(&mut C, crate::ui::background::Color) + 'static,
    set_gradient: impl Fn(&mut C, crate::ui::theme::LinearGradientSourceConfig) + 'static,
) -> (
    GtkBox,
    Rc<crate::ui::ColorButtonWidget>,
    Rc<crate::ui::ColorButtonWidget>,
    Rc<crate::ui::ColorButtonWidget>,
    Rc<crate::ui::ColorButtonWidget>,
    Rc<GradientEditor>,
)
where
    C: 'static,
{
    use crate::ui::ColorButtonWidget;

    let page = GtkBox::new(Orientation::Vertical, 8);
    set_page_margins(&page);

    // Theme Colors section
    let colors_label = Label::new(Some("Theme Colors"));
    colors_label.set_halign(gtk4::Align::Start);
    colors_label.add_css_class("heading");
    page.append(&colors_label);

    let theme = get_theme(&config.borrow());

    // Create a 2x2 grid for theme colors with proper alignment
    let colors_grid = gtk4::Grid::new();
    colors_grid.set_row_spacing(6);
    colors_grid.set_column_spacing(8);
    colors_grid.set_margin_start(6);

    // Color 1 (Primary) - row 0, col 0-1
    let color1_label = Label::new(Some("C1 (Primary):"));
    color1_label.set_halign(gtk4::Align::End);
    color1_label.set_width_chars(14);
    colors_grid.attach(&color1_label, 0, 0, 1, 1);
    let theme_color1_widget = Rc::new(ColorButtonWidget::new(theme.color1));
    colors_grid.attach(theme_color1_widget.widget(), 1, 0, 1, 1);

    // Color 2 (Secondary) - row 0, col 2-3
    let color2_label = Label::new(Some("C2 (Secondary):"));
    color2_label.set_halign(gtk4::Align::End);
    color2_label.set_width_chars(14);
    color2_label.set_margin_start(12);
    colors_grid.attach(&color2_label, 2, 0, 1, 1);
    let theme_color2_widget = Rc::new(ColorButtonWidget::new(theme.color2));
    colors_grid.attach(theme_color2_widget.widget(), 3, 0, 1, 1);

    // Color 3 (Accent) - row 1, col 0-1
    let color3_label = Label::new(Some("C3 (Accent):"));
    color3_label.set_halign(gtk4::Align::End);
    color3_label.set_width_chars(14);
    colors_grid.attach(&color3_label, 0, 1, 1, 1);
    let theme_color3_widget = Rc::new(ColorButtonWidget::new(theme.color3));
    colors_grid.attach(theme_color3_widget.widget(), 1, 1, 1, 1);

    // Color 4 (Highlight) - row 1, col 2-3
    let color4_label = Label::new(Some("C4 (Highlight):"));
    color4_label.set_halign(gtk4::Align::End);
    color4_label.set_width_chars(14);
    color4_label.set_margin_start(12);
    colors_grid.attach(&color4_label, 2, 1, 1, 1);
    let theme_color4_widget = Rc::new(ColorButtonWidget::new(theme.color4));
    colors_grid.attach(theme_color4_widget.widget(), 3, 1, 1, 1);

    page.append(&colors_grid);

    // Connect color widget callbacks
    let config_c1 = config.clone();
    let on_change_c1 = on_change.clone();
    let preview_c1 = preview.clone();
    let refreshers_c1 = theme_ref_refreshers.clone();
    theme_color1_widget.set_on_change(move |color| {
        set_color1(&mut config_c1.borrow_mut(), color);
        queue_redraw(&preview_c1, &on_change_c1);
        refresh_theme_refs(&refreshers_c1);
    });

    let config_c2 = config.clone();
    let on_change_c2 = on_change.clone();
    let preview_c2 = preview.clone();
    let refreshers_c2 = theme_ref_refreshers.clone();
    theme_color2_widget.set_on_change(move |color| {
        set_color2(&mut config_c2.borrow_mut(), color);
        queue_redraw(&preview_c2, &on_change_c2);
        refresh_theme_refs(&refreshers_c2);
    });

    let config_c3 = config.clone();
    let on_change_c3 = on_change.clone();
    let preview_c3 = preview.clone();
    let refreshers_c3 = theme_ref_refreshers.clone();
    theme_color3_widget.set_on_change(move |color| {
        set_color3(&mut config_c3.borrow_mut(), color);
        queue_redraw(&preview_c3, &on_change_c3);
        refresh_theme_refs(&refreshers_c3);
    });

    let config_c4 = config.clone();
    let on_change_c4 = on_change.clone();
    let preview_c4 = preview.clone();
    let refreshers_c4 = theme_ref_refreshers.clone();
    theme_color4_widget.set_on_change(move |color| {
        set_color4(&mut config_c4.borrow_mut(), color);
        queue_redraw(&preview_c4, &on_change_c4);
        refresh_theme_refs(&refreshers_c4);
    });

    // Theme Gradient section
    let gradient_label = Label::new(Some("Theme Gradient"));
    gradient_label.set_halign(gtk4::Align::Start);
    gradient_label.add_css_class("heading");
    gradient_label.set_margin_top(12);
    page.append(&gradient_label);

    let theme_gradient_editor = Rc::new(GradientEditor::new());
    theme_gradient_editor.set_gradient_source_config(&theme.gradient);
    page.append(theme_gradient_editor.widget());

    let config_grad = config.clone();
    let on_change_grad = on_change.clone();
    let preview_grad = preview.clone();
    let refreshers_grad = theme_ref_refreshers.clone();
    let gradient_editor_clone = theme_gradient_editor.clone();
    theme_gradient_editor.set_on_change(move || {
        set_gradient(&mut config_grad.borrow_mut(), gradient_editor_clone.get_gradient_source_config());
        queue_redraw(&preview_grad, &on_change_grad);
        refresh_theme_refs(&refreshers_grad);
    });

    (
        page,
        theme_color1_widget,
        theme_color2_widget,
        theme_color3_widget,
        theme_color4_widget,
        theme_gradient_editor,
    )
}
