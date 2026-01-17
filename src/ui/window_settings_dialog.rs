//! Window Settings Dialog
//!
//! Provides a dialog for configuring window-level settings:
//! - Grid settings (cell width, height, spacing)
//! - Default panel size and appearance
//! - Window background
//! - Fullscreen and borderless mode
//! - Auto-scroll settings

use gtk4::prelude::*;
use gtk4::ApplicationWindow;
use log::info;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::config::AppConfig;
use crate::ui::GridLayout;

/// Show window settings dialog
pub fn show_window_settings_dialog<F>(
    parent_window: &ApplicationWindow,
    app_config: &Rc<RefCell<AppConfig>>,
    window_background: &gtk4::DrawingArea,
    grid_layout: &Rc<RefCell<GridLayout>>,
    config_dirty: &Arc<AtomicBool>,
    on_auto_scroll_change: &Rc<F>,
) where
    F: Fn() + 'static,
{
    use crate::config::DefaultsConfig;
    use crate::ui::BackgroundConfigWidget;
    use gtk4::{
        Box as GtkBox, Button, CheckButton, DropDown, Label, Notebook, Orientation, SpinButton,
        StringList, Window,
    };

    let dialog = Window::builder()
        .title("Window Settings")
        .transient_for(parent_window)
        .modal(false)
        .default_width(550)
        .default_height(650)
        .build();

    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);

    // Create notebook for tabs
    let notebook = Notebook::new();
    notebook.set_vexpand(true);

    // Load current defaults
    let defaults_config = Rc::new(RefCell::new(DefaultsConfig::load()));

    // === Tab 1: Defaults (merged Grid Settings + Panel Defaults) ===
    let defaults_scroll = gtk4::ScrolledWindow::new();
    defaults_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    defaults_scroll.set_vexpand(true);

    let defaults_tab_box = GtkBox::new(Orientation::Vertical, 12);
    defaults_tab_box.set_margin_start(12);
    defaults_tab_box.set_margin_end(12);
    defaults_tab_box.set_margin_top(12);
    defaults_tab_box.set_margin_bottom(12);

    // --- Grid Settings Section ---
    let grid_section_label = Label::new(Some("Grid Settings"));
    grid_section_label.add_css_class("heading");
    grid_section_label.set_halign(gtk4::Align::Start);
    defaults_tab_box.append(&grid_section_label);

    // Cell Width
    let cell_width_box = GtkBox::new(Orientation::Horizontal, 6);
    cell_width_box.set_margin_start(12);
    cell_width_box.append(&Label::new(Some("Cell Width:")));
    let cell_width_spin = SpinButton::with_range(10.0, 1000.0, 10.0);
    cell_width_spin.set_value(app_config.borrow().grid.cell_width as f64);
    cell_width_spin.set_hexpand(true);
    cell_width_box.append(&cell_width_spin);
    cell_width_box.append(&Label::new(Some("px")));
    defaults_tab_box.append(&cell_width_box);

    // Cell Height
    let cell_height_box = GtkBox::new(Orientation::Horizontal, 6);
    cell_height_box.set_margin_start(12);
    cell_height_box.append(&Label::new(Some("Cell Height:")));
    let cell_height_spin = SpinButton::with_range(10.0, 1000.0, 10.0);
    cell_height_spin.set_value(app_config.borrow().grid.cell_height as f64);
    cell_height_spin.set_hexpand(true);
    cell_height_box.append(&cell_height_spin);
    cell_height_box.append(&Label::new(Some("px")));
    defaults_tab_box.append(&cell_height_box);

    // Spacing
    let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
    spacing_box.set_margin_start(12);
    spacing_box.append(&Label::new(Some("Spacing:")));
    let spacing_spin = SpinButton::with_range(0.0, 50.0, 1.0);
    spacing_spin.set_value(app_config.borrow().grid.spacing as f64);
    spacing_spin.set_hexpand(true);
    spacing_box.append(&spacing_spin);
    spacing_box.append(&Label::new(Some("px")));
    defaults_tab_box.append(&spacing_box);

    // --- Default Panel Size Section ---
    let panel_size_label = Label::new(Some("Default Panel Size"));
    panel_size_label.add_css_class("heading");
    panel_size_label.set_halign(gtk4::Align::Start);
    panel_size_label.set_margin_top(12);
    defaults_tab_box.append(&panel_size_label);

    let panel_size_help = Label::new(Some("Size in grid cells for new panels"));
    panel_size_help.set_halign(gtk4::Align::Start);
    panel_size_help.set_margin_start(12);
    panel_size_help.add_css_class("dim-label");
    defaults_tab_box.append(&panel_size_help);

    let panel_size_box = GtkBox::new(Orientation::Horizontal, 12);
    panel_size_box.set_margin_start(12);

    let panel_width_box = GtkBox::new(Orientation::Horizontal, 6);
    panel_width_box.append(&Label::new(Some("Width:")));
    let panel_width_spin = SpinButton::with_range(1.0, 20.0, 1.0);
    panel_width_spin.set_value(defaults_config.borrow().general.default_panel_width as f64);
    panel_width_box.append(&panel_width_spin);
    panel_width_box.append(&Label::new(Some("cells")));
    panel_size_box.append(&panel_width_box);

    let panel_height_box = GtkBox::new(Orientation::Horizontal, 6);
    panel_height_box.append(&Label::new(Some("Height:")));
    let panel_height_spin = SpinButton::with_range(1.0, 20.0, 1.0);
    panel_height_spin.set_value(defaults_config.borrow().general.default_panel_height as f64);
    panel_height_box.append(&panel_height_spin);
    panel_height_box.append(&Label::new(Some("cells")));
    panel_size_box.append(&panel_height_box);

    defaults_tab_box.append(&panel_size_box);

    // --- Default Panel Appearance Section ---
    let panel_appearance_label = Label::new(Some("Default Panel Appearance"));
    panel_appearance_label.add_css_class("heading");
    panel_appearance_label.set_halign(gtk4::Align::Start);
    panel_appearance_label.set_margin_top(12);
    defaults_tab_box.append(&panel_appearance_label);

    // Corner radius
    let corner_radius_box = GtkBox::new(Orientation::Horizontal, 6);
    corner_radius_box.set_margin_start(12);
    corner_radius_box.append(&Label::new(Some("Corner Radius:")));
    let corner_radius_spin = SpinButton::with_range(0.0, 50.0, 1.0);
    corner_radius_spin.set_value(defaults_config.borrow().general.default_corner_radius);
    corner_radius_spin.set_hexpand(true);
    corner_radius_box.append(&corner_radius_spin);
    defaults_tab_box.append(&corner_radius_box);

    // Border enabled
    let border_enabled_check = CheckButton::with_label("Show Border on New Panels");
    border_enabled_check.set_active(defaults_config.borrow().general.default_border.enabled);
    border_enabled_check.set_margin_start(12);
    defaults_tab_box.append(&border_enabled_check);

    // Border width
    let border_width_box = GtkBox::new(Orientation::Horizontal, 6);
    border_width_box.set_margin_start(12);
    border_width_box.append(&Label::new(Some("Border Width:")));
    let border_width_spin = SpinButton::with_range(0.5, 10.0, 0.5);
    border_width_spin.set_value(defaults_config.borrow().general.default_border.width);
    border_width_spin.set_hexpand(true);
    border_width_box.append(&border_width_spin);
    defaults_tab_box.append(&border_width_box);

    // Border color button
    let border_color_btn = Button::with_label("Border Color");
    border_color_btn.set_margin_start(12);
    defaults_tab_box.append(&border_color_btn);

    // Store border color in a shared Rc<RefCell>
    let border_color = Rc::new(RefCell::new(
        defaults_config.borrow().general.default_border.color,
    ));

    // Border color button handler
    {
        let border_color_clone = border_color.clone();
        let dialog_clone = dialog.clone();
        border_color_btn.connect_clicked(move |_| {
            let current_color = *border_color_clone.borrow();
            let window_opt = dialog_clone.clone().upcast::<Window>();
            let border_color_clone2 = border_color_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) =
                    crate::ui::ColorPickerDialog::pick_color(Some(&window_opt), current_color).await
                {
                    *border_color_clone2.borrow_mut() = new_color;
                }
            });
        });
    }

    // Default background
    let bg_label = Label::new(Some("Default Background:"));
    bg_label.set_halign(gtk4::Align::Start);
    bg_label.set_margin_start(12);
    bg_label.set_margin_top(6);
    defaults_tab_box.append(&bg_label);

    let default_bg_widget = BackgroundConfigWidget::new();
    default_bg_widget.set_theme_config(app_config.borrow().global_theme.clone());
    default_bg_widget.set_config(defaults_config.borrow().general.default_background.clone());
    let default_bg_widget_box = GtkBox::new(Orientation::Vertical, 0);
    default_bg_widget_box.set_margin_start(12);
    default_bg_widget_box.append(default_bg_widget.widget());
    defaults_tab_box.append(&default_bg_widget_box);
    let default_bg_widget = Rc::new(default_bg_widget);

    // --- Displayer Defaults Section ---
    let displayer_defaults_label = Label::new(Some("Saved Displayer Defaults"));
    displayer_defaults_label.add_css_class("heading");
    displayer_defaults_label.set_halign(gtk4::Align::Start);
    displayer_defaults_label.set_margin_top(12);
    defaults_tab_box.append(&displayer_defaults_label);

    let displayer_help = Label::new(Some(
        "Right-click a panel and select 'Set as Default Style' to save displayer defaults",
    ));
    displayer_help.set_halign(gtk4::Align::Start);
    displayer_help.set_margin_start(12);
    displayer_help.add_css_class("dim-label");
    displayer_help.set_wrap(true);
    defaults_tab_box.append(&displayer_help);

    // Container for displayer defaults list
    let displayer_list_box = GtkBox::new(Orientation::Vertical, 4);
    displayer_list_box.set_margin_start(12);
    displayer_list_box.set_margin_top(6);

    // Refresh function to populate the list
    let defaults_config_for_list = defaults_config.clone();
    let displayer_list_box_clone = displayer_list_box.clone();
    let refresh_displayer_list = Rc::new(RefCell::new(None::<Box<dyn Fn()>>));
    let refresh_displayer_list_clone = refresh_displayer_list.clone();

    let refresh_fn = {
        let defaults_config = defaults_config_for_list.clone();
        let list_box = displayer_list_box_clone.clone();
        let refresh_self = refresh_displayer_list_clone.clone();
        move || {
            // Clear existing children
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }

            let ids = defaults_config.borrow().get_displayer_default_ids();
            if ids.is_empty() {
                let no_defaults_label = Label::new(Some("No displayer defaults saved"));
                no_defaults_label.add_css_class("dim-label");
                no_defaults_label.set_halign(gtk4::Align::Start);
                list_box.append(&no_defaults_label);
            } else {
                for id in ids {
                    let row = GtkBox::new(Orientation::Horizontal, 6);

                    let id_label = Label::new(Some(&id));
                    id_label.set_hexpand(true);
                    id_label.set_halign(gtk4::Align::Start);
                    row.append(&id_label);

                    let clear_btn = Button::with_label("Clear");
                    clear_btn.add_css_class("destructive-action");
                    let defaults_clone = defaults_config.clone();
                    let id_clone = id.clone();
                    let refresh_clone = refresh_self.clone();
                    clear_btn.connect_clicked(move |_| {
                        defaults_clone
                            .borrow_mut()
                            .remove_displayer_default(&id_clone);
                        if let Err(e) = defaults_clone.borrow().save() {
                            log::warn!("Failed to save defaults after clearing: {}", e);
                        }
                        // Refresh the list
                        if let Some(ref f) = *refresh_clone.borrow() {
                            f();
                        }
                    });
                    row.append(&clear_btn);

                    list_box.append(&row);
                }
            }
        }
    };

    // Store the refresh function
    *refresh_displayer_list.borrow_mut() = Some(Box::new(refresh_fn.clone()));

    // Initial population
    refresh_fn();

    defaults_tab_box.append(&displayer_list_box);

    // Clear All button
    let clear_all_btn = Button::with_label("Clear All Displayer Defaults");
    clear_all_btn.add_css_class("destructive-action");
    clear_all_btn.set_margin_start(12);
    clear_all_btn.set_margin_top(6);
    clear_all_btn.set_halign(gtk4::Align::Start);
    let defaults_for_clear_all = defaults_config.clone();
    let refresh_for_clear_all = refresh_displayer_list.clone();
    clear_all_btn.connect_clicked(move |_| {
        defaults_for_clear_all
            .borrow_mut()
            .clear_displayer_defaults();
        if let Err(e) = defaults_for_clear_all.borrow().save() {
            log::warn!("Failed to save defaults after clearing all: {}", e);
        }
        // Refresh the list
        if let Some(ref f) = *refresh_for_clear_all.borrow() {
            f();
        }
    });
    defaults_tab_box.append(&clear_all_btn);

    defaults_scroll.set_child(Some(&defaults_tab_box));
    notebook.append_page(&defaults_scroll, Some(&Label::new(Some("Defaults"))));

    // === Tab 2: Background ===
    let bg_tab_box = GtkBox::new(Orientation::Vertical, 12);
    bg_tab_box.set_margin_start(12);
    bg_tab_box.set_margin_end(12);
    bg_tab_box.set_margin_top(12);
    bg_tab_box.set_margin_bottom(12);

    let background_widget = BackgroundConfigWidget::new();
    background_widget.set_theme_config(app_config.borrow().global_theme.clone());
    background_widget.set_config(app_config.borrow().window.background.clone());
    bg_tab_box.append(background_widget.widget());

    let background_widget = Rc::new(background_widget);

    notebook.append_page(&bg_tab_box, Some(&Label::new(Some("Background"))));

    // === Tab 3: Window Mode ===
    // Wrap in ScrolledWindow since this tab has a lot of content
    let window_mode_scroll = gtk4::ScrolledWindow::new();
    window_mode_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    window_mode_scroll.set_vexpand(true);

    let window_mode_tab_box = GtkBox::new(Orientation::Vertical, 12);
    window_mode_tab_box.set_margin_start(12);
    window_mode_tab_box.set_margin_end(12);
    window_mode_tab_box.set_margin_top(12);
    window_mode_tab_box.set_margin_bottom(12);

    // Fullscreen section
    let fullscreen_label = Label::new(Some("Fullscreen"));
    fullscreen_label.add_css_class("heading");
    fullscreen_label.set_halign(gtk4::Align::Start);
    window_mode_tab_box.append(&fullscreen_label);

    // Fullscreen enabled
    let fullscreen_enabled_check = CheckButton::with_label("Start in fullscreen mode");
    fullscreen_enabled_check.set_active(app_config.borrow().window.fullscreen_enabled);
    fullscreen_enabled_check.set_margin_start(12);
    window_mode_tab_box.append(&fullscreen_enabled_check);

    // Fullscreen monitor selection
    let monitor_box = GtkBox::new(Orientation::Horizontal, 6);
    monitor_box.set_margin_start(12);
    monitor_box.append(&Label::new(Some("Monitor:")));

    // Get list of available monitors with their names
    let monitor_names = if let Some(display) = gtk4::gdk::Display::default() {
        let n_monitors = display.monitors().n_items();
        (0..n_monitors)
            .filter_map(|i| {
                display
                    .monitors()
                    .item(i)
                    .and_then(|obj| obj.downcast::<gtk4::gdk::Monitor>().ok())
                    .map(|monitor| {
                        // Try to get connector name (e.g., "HDMI-1", "DP-1")
                        let connector = monitor
                            .connector()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("Monitor {}", i));

                        // Get model name if available
                        let model = monitor.model().map(|s| s.to_string());

                        // Combine connector and model for a descriptive name
                        match model {
                            Some(m) if !m.is_empty() => format!("{} ({})", connector, m),
                            _ => connector,
                        }
                    })
            })
            .collect::<Vec<_>>()
    } else {
        vec!["Monitor 0".to_string()]
    };

    let mut monitor_strings: Vec<String> = vec!["Current Monitor".to_string()];
    monitor_strings.extend(monitor_names);

    let monitor_string_refs: Vec<&str> = monitor_strings.iter().map(|s| s.as_str()).collect();
    let monitor_list = StringList::new(&monitor_string_refs);
    let monitor_dropdown = DropDown::new(Some(monitor_list), Option::<gtk4::Expression>::None);
    monitor_dropdown.set_hexpand(true);

    // Set selected monitor from config
    let selected_idx = match app_config.borrow().window.fullscreen_monitor {
        None => 0,                     // "Current Monitor"
        Some(idx) => (idx + 1) as u32, // Offset by 1 for "Current Monitor" option
    };
    monitor_dropdown.set_selected(selected_idx);
    monitor_box.append(&monitor_dropdown);
    window_mode_tab_box.append(&monitor_box);

    // Help text for fullscreen
    let fullscreen_help_label = Label::new(Some(
        "Tip: Double-click the window background to toggle fullscreen",
    ));
    fullscreen_help_label.set_halign(gtk4::Align::Start);
    fullscreen_help_label.set_margin_start(12);
    fullscreen_help_label.set_margin_top(6);
    fullscreen_help_label.add_css_class("dim-label");
    window_mode_tab_box.append(&fullscreen_help_label);

    // Borderless section
    let borderless_label = Label::new(Some("Borderless Mode"));
    borderless_label.add_css_class("heading");
    borderless_label.set_halign(gtk4::Align::Start);
    borderless_label.set_margin_top(18);
    window_mode_tab_box.append(&borderless_label);

    // Borderless enabled
    let borderless_check =
        CheckButton::with_label("Remove window decorations (title bar, borders)");
    borderless_check.set_active(app_config.borrow().window.borderless);
    borderless_check.set_margin_start(12);
    window_mode_tab_box.append(&borderless_check);

    // Info box for borderless mode
    let borderless_info_frame = gtk4::Frame::new(None);
    borderless_info_frame.set_margin_start(12);
    borderless_info_frame.set_margin_top(6);
    borderless_info_frame.add_css_class("view");

    let borderless_info_box = GtkBox::new(Orientation::Horizontal, 8);
    borderless_info_box.set_margin_start(8);
    borderless_info_box.set_margin_end(8);
    borderless_info_box.set_margin_top(8);
    borderless_info_box.set_margin_bottom(8);

    let info_icon = Label::new(Some("\u{2139}")); // ℹ info symbol
    info_icon.add_css_class("dim-label");
    borderless_info_box.append(&info_icon);

    let borderless_info_label = Label::new(Some(
        "When borderless mode is active, hold Ctrl and drag:\n\
         • Near edges/corners to resize the window\n\
         • In center area to move the window",
    ));
    borderless_info_label.set_halign(gtk4::Align::Start);
    borderless_info_label.set_wrap(true);
    borderless_info_label.add_css_class("dim-label");
    borderless_info_box.append(&borderless_info_label);

    borderless_info_frame.set_child(Some(&borderless_info_box));

    // Show/hide info based on checkbox state
    borderless_info_frame.set_visible(borderless_check.is_active());
    let borderless_info_frame_clone = borderless_info_frame.clone();
    borderless_check.connect_toggled(move |check| {
        borderless_info_frame_clone.set_visible(check.is_active());
    });

    window_mode_tab_box.append(&borderless_info_frame);

    // Auto-scroll section
    let auto_scroll_label = Label::new(Some("Auto-Scroll"));
    auto_scroll_label.add_css_class("heading");
    auto_scroll_label.set_halign(gtk4::Align::Start);
    auto_scroll_label.set_margin_top(18);
    window_mode_tab_box.append(&auto_scroll_label);

    // Auto-scroll enabled
    let auto_scroll_check =
        CheckButton::with_label("Auto-scroll when content extends beyond window");
    auto_scroll_check.set_active(app_config.borrow().window.auto_scroll_enabled);
    auto_scroll_check.set_margin_start(12);
    window_mode_tab_box.append(&auto_scroll_check);

    // Auto-scroll delay
    let delay_box = GtkBox::new(Orientation::Horizontal, 6);
    delay_box.set_margin_start(12);
    delay_box.append(&Label::new(Some("Scroll delay:")));

    let delay_spin = SpinButton::with_range(500.0, 60000.0, 500.0);
    delay_spin.set_value(app_config.borrow().window.auto_scroll_delay_ms as f64);
    delay_spin.set_hexpand(true);
    delay_spin.set_sensitive(auto_scroll_check.is_active());
    delay_box.append(&delay_spin);
    delay_box.append(&Label::new(Some("ms")));
    window_mode_tab_box.append(&delay_box);

    // Whole pages checkbox
    let whole_pages_check = CheckButton::with_label("Scroll whole pages only");
    whole_pages_check.set_active(app_config.borrow().window.auto_scroll_whole_pages);
    whole_pages_check.set_margin_start(12);
    whole_pages_check.set_sensitive(auto_scroll_check.is_active());
    window_mode_tab_box.append(&whole_pages_check);

    // Enable/disable delay spin and whole pages based on checkbox
    let delay_spin_clone = delay_spin.clone();
    let whole_pages_check_clone = whole_pages_check.clone();
    auto_scroll_check.connect_toggled(move |check| {
        delay_spin_clone.set_sensitive(check.is_active());
        whole_pages_check_clone.set_sensitive(check.is_active());
    });

    // Auto-scroll help text
    let auto_scroll_help = Label::new(Some("Scrolls one viewport page at a time. When 'whole pages only' is enabled, scrolls through complete page grid regardless of panel positions."));
    auto_scroll_help.set_halign(gtk4::Align::Start);
    auto_scroll_help.set_margin_start(12);
    auto_scroll_help.set_margin_top(6);
    auto_scroll_help.add_css_class("dim-label");
    auto_scroll_help.set_wrap(true);
    window_mode_tab_box.append(&auto_scroll_help);

    // Viewport dimensions for auto-scroll page boundaries
    let viewport_label = Label::new(Some("Viewport Page Dimensions"));
    viewport_label.set_halign(gtk4::Align::Start);
    viewport_label.set_margin_top(12);
    viewport_label.add_css_class("heading");
    window_mode_tab_box.append(&viewport_label);

    let viewport_help = Label::new(Some("Define the page size for auto-scroll boundaries. Shown as dashed rectangles when dragging panels."));
    viewport_help.set_halign(gtk4::Align::Start);
    viewport_help.set_margin_start(12);
    viewport_help.add_css_class("dim-label");
    viewport_help.set_wrap(true);
    window_mode_tab_box.append(&viewport_help);

    // Viewport width/height inputs
    let viewport_dims_box = GtkBox::new(Orientation::Horizontal, 12);
    viewport_dims_box.set_margin_start(12);
    viewport_dims_box.set_margin_top(6);

    let vp_width_box = GtkBox::new(Orientation::Horizontal, 4);
    vp_width_box.append(&Label::new(Some("Width:")));
    let viewport_width_spin = SpinButton::with_range(0.0, 10000.0, 10.0);
    viewport_width_spin.set_value(app_config.borrow().window.viewport_width as f64);
    viewport_width_spin.set_width_chars(6);
    vp_width_box.append(&viewport_width_spin);
    vp_width_box.append(&Label::new(Some("px")));
    viewport_dims_box.append(&vp_width_box);

    let vp_height_box = GtkBox::new(Orientation::Horizontal, 4);
    vp_height_box.append(&Label::new(Some("Height:")));
    let viewport_height_spin = SpinButton::with_range(0.0, 10000.0, 10.0);
    viewport_height_spin.set_value(app_config.borrow().window.viewport_height as f64);
    viewport_height_spin.set_width_chars(6);
    vp_height_box.append(&viewport_height_spin);
    vp_height_box.append(&Label::new(Some("px")));
    viewport_dims_box.append(&vp_height_box);

    window_mode_tab_box.append(&viewport_dims_box);

    // Copy buttons
    let copy_buttons_box = GtkBox::new(Orientation::Horizontal, 6);
    copy_buttons_box.set_margin_start(12);
    copy_buttons_box.set_margin_top(6);

    let copy_window_btn = Button::with_label("Copy from Window");
    let copy_monitor_btn = Button::with_label("Copy from Monitor");
    let apply_to_window_btn = Button::with_label("Apply to Window");
    apply_to_window_btn.set_tooltip_text(Some("Resize main window to viewport dimensions"));

    // Monitor dropdown for copy
    let monitor_list = StringList::new(&[]);
    if let Some(display) = gtk4::gdk::Display::default() {
        let monitors = display.monitors();
        for i in 0..monitors.n_items() {
            if let Some(mon) = monitors.item(i) {
                if let Ok(monitor) = mon.downcast::<gtk4::gdk::Monitor>() {
                    let geom = monitor.geometry();
                    let name = monitor
                        .model()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("Monitor {}", i));
                    monitor_list.append(&format!("{} ({}x{})", name, geom.width(), geom.height()));
                }
            }
        }
    }
    let vp_monitor_dropdown = DropDown::new(Some(monitor_list.clone()), None::<gtk4::Expression>);
    vp_monitor_dropdown.set_selected(0);

    copy_buttons_box.append(&copy_window_btn);
    copy_buttons_box.append(&copy_monitor_btn);
    copy_buttons_box.append(&vp_monitor_dropdown);
    copy_buttons_box.append(&apply_to_window_btn);
    window_mode_tab_box.append(&copy_buttons_box);

    // Connect copy from window button
    {
        let parent_clone = parent_window.clone();
        let vp_width = viewport_width_spin.clone();
        let vp_height = viewport_height_spin.clone();
        copy_window_btn.connect_clicked(move |_| {
            vp_width.set_value(parent_clone.width() as f64);
            vp_height.set_value(parent_clone.height() as f64);
        });
    }

    // Connect copy from monitor button
    {
        let vp_width = viewport_width_spin.clone();
        let vp_height = viewport_height_spin.clone();
        let monitor_dd = vp_monitor_dropdown.clone();
        copy_monitor_btn.connect_clicked(move |_| {
            if let Some(display) = gtk4::gdk::Display::default() {
                let selected = monitor_dd.selected();
                if let Some(mon) = display.monitors().item(selected) {
                    if let Ok(monitor) = mon.downcast::<gtk4::gdk::Monitor>() {
                        let geom = monitor.geometry();
                        vp_width.set_value(geom.width() as f64);
                        vp_height.set_value(geom.height() as f64);
                    }
                }
            }
        });
    }

    // Connect apply to window button - resize main window to viewport dimensions
    {
        let parent_clone = parent_window.clone();
        let vp_width = viewport_width_spin.clone();
        let vp_height = viewport_height_spin.clone();
        apply_to_window_btn.connect_clicked(move |_| {
            let width = vp_width.value() as i32;
            let height = vp_height.value() as i32;
            if width > 0 && height > 0 {
                parent_clone.set_default_size(width, height);
                // Queue a resize - this helps ensure the window updates
                parent_clone.queue_resize();
            }
        });
    }

    // Zero = use window size help text
    let vp_zero_help = Label::new(Some("Set to 0 to use current window dimensions"));
    vp_zero_help.set_halign(gtk4::Align::Start);
    vp_zero_help.set_margin_start(12);
    vp_zero_help.set_margin_top(4);
    vp_zero_help.add_css_class("dim-label");
    window_mode_tab_box.append(&vp_zero_help);

    // Grid overlay shortcuts help
    let grid_shortcuts_label = Label::new(Some("Grid Overlay Shortcuts"));
    grid_shortcuts_label.set_halign(gtk4::Align::Start);
    grid_shortcuts_label.set_margin_top(12);
    grid_shortcuts_label.add_css_class("heading");
    window_mode_tab_box.append(&grid_shortcuts_label);

    let grid_shortcuts_help = Label::new(Some(
        "• Hold Space to show the cell grid and viewport boundaries\n\
         • Grid also appears automatically when resizing the window\n\
         • Grid appears when dragging panels",
    ));
    grid_shortcuts_help.set_halign(gtk4::Align::Start);
    grid_shortcuts_help.set_margin_start(12);
    grid_shortcuts_help.set_margin_top(4);
    grid_shortcuts_help.add_css_class("dim-label");
    grid_shortcuts_help.set_wrap(true);
    window_mode_tab_box.append(&grid_shortcuts_help);

    // Set the scrolled window content and add to notebook
    window_mode_scroll.set_child(Some(&window_mode_tab_box));
    notebook.append_page(&window_mode_scroll, Some(&Label::new(Some("Window Mode"))));

    // === Tab 4: Global Theme ===
    let theme_scroll = gtk4::ScrolledWindow::new();
    theme_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    theme_scroll.set_vexpand(true);

    let global_theme_widget = Rc::new(crate::ui::GlobalThemeWidget::new());
    global_theme_widget.set_config(app_config.borrow().global_theme.clone());
    theme_scroll.set_child(Some(global_theme_widget.widget()));

    notebook.append_page(&theme_scroll, Some(&Label::new(Some("Global Theme"))));

    vbox.append(&notebook);

    // Buttons
    let button_box = GtkBox::new(Orientation::Horizontal, 6);
    button_box.set_halign(gtk4::Align::End);
    button_box.set_margin_top(12);

    let cancel_button = Button::with_label("Cancel");
    let apply_button = Button::with_label("Apply");
    let accept_button = Button::with_label("Accept");
    accept_button.add_css_class("suggested-action");

    let dialog_clone = dialog.clone();
    cancel_button.connect_clicked(move |_| {
        dialog_clone.close();
    });

    // Apply logic
    let app_config_clone = app_config.clone();
    let background_widget_clone = background_widget.clone();
    let window_background_clone = window_background.clone();
    let grid_layout_clone = grid_layout.clone();
    let config_dirty_clone = config_dirty.clone();
    let corner_radius_spin_clone = corner_radius_spin.clone();
    let border_enabled_check_clone = border_enabled_check.clone();
    let border_width_spin_clone = border_width_spin.clone();
    let border_color_clone = border_color.clone();
    let fullscreen_enabled_check_clone = fullscreen_enabled_check.clone();
    let monitor_dropdown_clone = monitor_dropdown.clone();
    let borderless_check_clone = borderless_check.clone();
    let auto_scroll_check_clone = auto_scroll_check.clone();
    let delay_spin_clone = delay_spin.clone();
    let whole_pages_check_clone = whole_pages_check.clone();
    let viewport_width_spin_clone = viewport_width_spin.clone();
    let viewport_height_spin_clone = viewport_height_spin.clone();
    let parent_window_clone = parent_window.clone();
    let on_auto_scroll_change_clone = on_auto_scroll_change.clone();
    // Clones for defaults
    let defaults_config_clone = defaults_config.clone();
    let panel_width_spin_clone = panel_width_spin.clone();
    let panel_height_spin_clone = panel_height_spin.clone();
    let default_bg_widget_clone = default_bg_widget.clone();
    // Clone for global theme
    let global_theme_widget_clone = global_theme_widget.clone();

    let apply_changes = Rc::new(move || {
        let new_background = background_widget_clone.get_config();
        let new_cell_width = cell_width_spin.value() as i32;
        let new_cell_height = cell_height_spin.value() as i32;
        let new_spacing = spacing_spin.value() as i32;

        // Get fullscreen settings
        let fullscreen_enabled = fullscreen_enabled_check_clone.is_active();
        let fullscreen_monitor = {
            let selected = monitor_dropdown_clone.selected();
            if selected == 0 {
                None // "Current Monitor"
            } else {
                Some((selected - 1) as i32) // Offset by 1 for "Current Monitor" option
            }
        };

        // Get borderless setting
        let borderless = borderless_check_clone.is_active();

        // Update defaults.json with panel defaults
        {
            let mut defaults = defaults_config_clone.borrow_mut();
            defaults.general.grid_cell_width = new_cell_width as u32;
            defaults.general.grid_cell_height = new_cell_height as u32;
            defaults.general.grid_spacing = new_spacing as u32;
            defaults.general.default_panel_width = panel_width_spin_clone.value() as u32;
            defaults.general.default_panel_height = panel_height_spin_clone.value() as u32;
            defaults.general.default_corner_radius = corner_radius_spin_clone.value();
            defaults.general.default_border.enabled = border_enabled_check_clone.is_active();
            defaults.general.default_border.width = border_width_spin_clone.value();
            defaults.general.default_border.color = *border_color_clone.borrow();
            defaults.general.default_background = default_bg_widget_clone.get_config();
            if let Err(e) = defaults.save() {
                log::warn!("Failed to save defaults.json: {}", e);
            }
        }

        // Update app config
        let mut cfg = app_config_clone.borrow_mut();
        cfg.window.background = new_background.clone();
        cfg.grid.cell_width = new_cell_width;
        cfg.grid.cell_height = new_cell_height;
        cfg.grid.spacing = new_spacing;
        // Also update AppConfig panel defaults for backward compatibility
        cfg.window.panel_corner_radius = corner_radius_spin_clone.value();
        cfg.window.panel_border.enabled = border_enabled_check_clone.is_active();
        cfg.window.panel_border.width = border_width_spin_clone.value();
        cfg.window.panel_border.color = *border_color_clone.borrow();
        cfg.window.fullscreen_enabled = fullscreen_enabled;
        cfg.window.fullscreen_monitor = fullscreen_monitor;
        cfg.window.borderless = borderless;
        cfg.window.auto_scroll_enabled = auto_scroll_check_clone.is_active();
        cfg.window.auto_scroll_delay_ms = delay_spin_clone.value() as u64;
        cfg.window.auto_scroll_whole_pages = whole_pages_check_clone.is_active();
        cfg.window.viewport_width = viewport_width_spin_clone.value() as i32;
        cfg.window.viewport_height = viewport_height_spin_clone.value() as i32;
        // Save global theme
        cfg.global_theme = global_theme_widget_clone.get_config();

        // Calculate effective viewport size (use window size if set to 0)
        let vp_width = if cfg.window.viewport_width > 0 {
            cfg.window.viewport_width
        } else {
            parent_window_clone.width()
        };
        let vp_height = if cfg.window.viewport_height > 0 {
            cfg.window.viewport_height
        } else {
            parent_window_clone.height()
        };
        drop(cfg);

        // Update grid layout viewport size for drag visualization
        grid_layout_clone
            .borrow()
            .set_viewport_size(vp_width, vp_height);

        // Apply borderless state to parent window
        // Note: On some compositors (especially Wayland), decoration changes require
        // hiding and showing the window for them to take effect
        let current_decorated = parent_window_clone.is_decorated();
        let target_decorated = !borderless;
        log::info!(
            "Setting window decorated: {} -> {} (borderless: {})",
            current_decorated,
            target_decorated,
            borderless
        );

        if current_decorated != target_decorated {
            parent_window_clone.set_decorated(target_decorated);
            // Force window manager to apply decoration change
            parent_window_clone.queue_draw();
            parent_window_clone.present();
        }

        // Apply fullscreen state to parent window
        if fullscreen_enabled {
            if let Some(monitor) = fullscreen_monitor {
                // Fullscreen on specific monitor
                if let Some(display) = gtk4::gdk::Display::default() {
                    if let Some(mon) = display.monitors().item(monitor as u32) {
                        if let Ok(monitor) = mon.downcast::<gtk4::gdk::Monitor>() {
                            parent_window_clone.fullscreen_on_monitor(&monitor);
                        }
                    }
                }
            } else {
                // Fullscreen on current monitor
                parent_window_clone.fullscreen();
            }
        } else {
            parent_window_clone.unfullscreen();
        }

        // Trigger window background redraw (draw func reads config dynamically)
        window_background_clone.queue_draw();

        // Update grid layout
        grid_layout_clone.borrow_mut().update_grid_size(
            new_cell_width,
            new_cell_height,
            new_spacing,
        );

        // Apply global theme to all panel displayers
        {
            let global_theme = app_config_clone.borrow().global_theme.clone();
            let theme_value = serde_json::to_value(&global_theme).unwrap_or_default();
            let mut theme_config = std::collections::HashMap::new();
            theme_config.insert("global_theme".to_string(), theme_value);

            let panels = grid_layout_clone.borrow().get_panels();
            for panel in &panels {
                if let Ok(mut panel_guard) = panel.try_write() {
                    let _ = panel_guard.displayer.apply_config(&theme_config);
                }
            }
        }

        // Trigger all panels to redraw with new theme
        grid_layout_clone.borrow().queue_redraw_all_panels();

        // Mark config as dirty
        config_dirty_clone.store(true, Ordering::Relaxed);

        // Restart auto-scroll timer with new settings
        on_auto_scroll_change_clone();

        info!("Window settings applied");
    });

    // Apply button
    let apply_changes_clone = apply_changes.clone();
    apply_button.connect_clicked(move |_| {
        apply_changes_clone();
    });

    // Accept button
    let apply_changes_clone2 = apply_changes.clone();
    let dialog_clone2 = dialog.clone();
    accept_button.connect_clicked(move |_| {
        apply_changes_clone2();
        dialog_clone2.close();
    });

    button_box.append(&cancel_button);
    button_box.append(&apply_button);
    button_box.append(&accept_button);

    vbox.append(&button_box);

    dialog.set_child(Some(&vbox));
    dialog.present();
}
