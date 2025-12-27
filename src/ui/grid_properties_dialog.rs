//! Panel properties dialog for configuring panel settings
//!
//! This module contains the large panel properties dialog that was extracted
//! from grid_layout.rs for better code organization.

use crate::core::Panel;
use gtk4::glib::WeakRef;
use gtk4::prelude::*;
use gtk4::{DrawingArea, Fixed, Window};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::grid_layout::{delete_selected_panels, GridConfig, PanelState};

thread_local! {
    /// Singleton reference to the panel properties dialog
    pub(crate) static PANEL_PROPERTIES_DIALOG: RefCell<Option<WeakRef<Window>>> = const { RefCell::new(None) };
}

/// Close the panel properties dialog if it's open
pub fn close_panel_properties_dialog() {
    PANEL_PROPERTIES_DIALOG.with(|dialog_ref| {
        let mut dialog_opt = dialog_ref.borrow_mut();
        if let Some(weak) = dialog_opt.take() {
            if let Some(dialog) = weak.upgrade() {
                dialog.close();
            }
        }
    });
}

/// Show panel properties dialog
pub(crate) fn show_panel_properties_dialog(
    panel: &Arc<RwLock<Panel>>,
    config: GridConfig,
    panel_states: Rc<RefCell<HashMap<String, PanelState>>>,
    occupied_cells: Rc<RefCell<HashSet<(u32, u32)>>>,
    _container: Fixed,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    drop_zone: DrawingArea,
    registry: &'static crate::core::Registry,
    selected_panels: Rc<RefCell<HashSet<String>>>,
    panels: Rc<RefCell<Vec<Arc<RwLock<Panel>>>>>,
) {
    use gtk4::{Box as GtkBox, Button, DropDown, Label, Notebook, Orientation, SpinButton, StringList, Window};

    // Use blocking read - the update thread should release quickly
    let panel_guard = match panel.try_read() {
        Ok(guard) => guard,
        Err(_) => {
            // Panel is locked, use blocking read (updates are fast so this should be quick)
            log::info!("Panel locked, waiting for access...");
            panel.blocking_read()
        }
    };

    let panel_id = panel_guard.id.clone();
    let old_geometry = Rc::new(RefCell::new(panel_guard.geometry));
    let old_source_id = panel_guard.source.metadata().id.clone();
    let old_displayer_id = panel_guard.displayer.id().to_string();

    // Get parent window for transient_for
    let parent_window = _container.root().and_then(|r| r.downcast::<Window>().ok());

    // Get the ScrolledWindow from parent to preserve scroll position when dialog opens
    // The window child hierarchy is: Window -> ScrolledWindow -> Overlay -> ...
    let scrolled_window = parent_window.as_ref()
        .and_then(|w| w.child())
        .and_then(|c| c.downcast::<gtk4::ScrolledWindow>().ok());

    // Save current scroll position before showing dialog
    let saved_scroll = scrolled_window.as_ref().map(|sw| {
        (sw.hadjustment().value(), sw.vadjustment().value())
    });

    // Create dialog window
    let dialog = Window::builder()
        .title(format!("Panel Properties - {}", panel_id))
        .modal(false)
        .default_width(550)
        .default_height(650)
        .build();

    // Set transient for parent window so dialog stays on top
    if let Some(ref parent) = parent_window {
        dialog.set_transient_for(Some(parent));
    }

    // Close any existing panel properties dialog (singleton pattern)
    // Note: We must extract the existing dialog BEFORE closing it, because
    // close() triggers connect_close_request which also borrows PANEL_PROPERTIES_DIALOG
    let existing_dialog = PANEL_PROPERTIES_DIALOG.with(|dialog_ref| {
        let dialog_opt = dialog_ref.borrow();
        dialog_opt.as_ref().and_then(|weak| weak.upgrade())
    });
    if let Some(existing) = existing_dialog {
        existing.close();
    }
    // Now store the new dialog reference
    PANEL_PROPERTIES_DIALOG.with(|dialog_ref| {
        *dialog_ref.borrow_mut() = Some(dialog.downgrade());
    });

    // Main container
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);

    // Create notebook for tabs
    let notebook = Notebook::new();
    notebook.set_vexpand(true);

    // === Tab 1: Panel Properties ===
    let panel_props_box = GtkBox::new(Orientation::Vertical, 12);
    panel_props_box.set_margin_top(12);
    panel_props_box.set_margin_bottom(12);
    panel_props_box.set_margin_start(12);
    panel_props_box.set_margin_end(12);

    // Panel Size section
    let size_label = Label::new(Some("Panel Size"));
    size_label.add_css_class("heading");
    panel_props_box.append(&size_label);

    let size_box = GtkBox::new(Orientation::Horizontal, 6);
    size_box.set_margin_start(12);

    // Width control
    let width_label = Label::new(Some("Width:"));
    let width_spin = SpinButton::with_range(1.0, 512.0, 1.0);
    width_spin.set_value(old_geometry.borrow().width as f64);

    // Height control
    let height_label = Label::new(Some("Height:"));
    let height_spin = SpinButton::with_range(1.0, 512.0, 1.0);
    height_spin.set_value(old_geometry.borrow().height as f64);

    size_box.append(&width_label);
    size_box.append(&width_spin);
    size_box.append(&height_label);
    size_box.append(&height_spin);

    panel_props_box.append(&size_box);

    // Content Transform section
    let transform_label = Label::new(Some("Content Transform"));
    transform_label.add_css_class("heading");
    transform_label.set_margin_top(12);
    panel_props_box.append(&transform_label);

    // Scale control
    let scale_box = GtkBox::new(Orientation::Horizontal, 6);
    scale_box.set_margin_start(12);
    let scale_label = Label::new(Some("Scale:"));
    let scale_spin = SpinButton::with_range(0.1, 5.0, 0.1);
    scale_spin.set_digits(2);
    scale_spin.set_value(panel_guard.scale);
    scale_spin.set_hexpand(true);
    scale_box.append(&scale_label);
    scale_box.append(&scale_spin);
    panel_props_box.append(&scale_box);

    // Translation controls
    let translate_box = GtkBox::new(Orientation::Horizontal, 6);
    translate_box.set_margin_start(12);
    let translate_x_label = Label::new(Some("Offset X:"));
    let translate_x_spin = SpinButton::with_range(-500.0, 500.0, 1.0);
    translate_x_spin.set_digits(1);
    translate_x_spin.set_value(panel_guard.translate_x);
    let translate_y_label = Label::new(Some("Y:"));
    let translate_y_spin = SpinButton::with_range(-500.0, 500.0, 1.0);
    translate_y_spin.set_digits(1);
    translate_y_spin.set_value(panel_guard.translate_y);
    translate_box.append(&translate_x_label);
    translate_box.append(&translate_x_spin);
    translate_box.append(&translate_y_label);
    translate_box.append(&translate_y_spin);
    panel_props_box.append(&translate_box);

    // Panel Layering section
    let layering_label = Label::new(Some("Panel Layering"));
    layering_label.add_css_class("heading");
    layering_label.set_margin_top(12);
    panel_props_box.append(&layering_label);

    // Z-Index control
    let z_index_box = GtkBox::new(Orientation::Horizontal, 6);
    z_index_box.set_margin_start(12);
    let z_index_label = Label::new(Some("Z-Index:"));
    let z_index_spin = SpinButton::with_range(-100.0, 100.0, 1.0);
    z_index_spin.set_value(panel_guard.z_index as f64);
    z_index_spin.set_hexpand(true);
    z_index_spin.set_tooltip_text(Some("Higher values bring the panel in front of others"));
    z_index_box.append(&z_index_label);
    z_index_box.append(&z_index_spin);
    panel_props_box.append(&z_index_box);

    // Ignore Collision control
    let collision_box = GtkBox::new(Orientation::Horizontal, 6);
    collision_box.set_margin_start(12);
    let ignore_collision_check = gtk4::CheckButton::with_label("Ignore collision (allow overlap)");
    ignore_collision_check.set_active(panel_guard.ignore_collision);
    ignore_collision_check.set_tooltip_text(Some("When enabled, this panel can overlap with other panels"));
    collision_box.append(&ignore_collision_check);
    panel_props_box.append(&collision_box);

    notebook.append_page(&panel_props_box, Some(&Label::new(Some("Size"))));

    // === Tab 2: Data Source ===
    let source_tab_box = GtkBox::new(Orientation::Vertical, 12);
    source_tab_box.set_margin_top(12);
    source_tab_box.set_margin_bottom(12);
    source_tab_box.set_margin_start(12);
    source_tab_box.set_margin_end(12);

    let source_label = Label::new(Some("Data Source"));
    source_label.add_css_class("heading");
    source_tab_box.append(&source_label);

    let source_box = GtkBox::new(Orientation::Horizontal, 6);
    source_box.set_margin_start(12);

    let source_combo_label = Label::new(Some("Source:"));

    // Populate source dropdown
    let sources = registry.list_sources();
    let mut selected_source_idx = 0;
    for (idx, source_id) in sources.iter().enumerate() {
        if source_id == &old_source_id {
            selected_source_idx = idx;
        }
    }

    let source_strings: Vec<&str> = sources.iter().map(|s| s.as_str()).collect();
    let source_list = StringList::new(&source_strings);
    let source_combo = DropDown::new(Some(source_list), Option::<gtk4::Expression>::None);
    source_combo.set_selected(selected_source_idx as u32);

    source_box.append(&source_combo_label);
    source_box.append(&source_combo);
    source_tab_box.append(&source_box);

    // CPU source configuration widget
    let cpu_config_widget = crate::ui::CpuSourceConfigWidget::new();
    cpu_config_widget.widget().set_visible(old_source_id == "cpu");

    // Populate sensor and core information from cached CPU hardware info
    cpu_config_widget.set_available_sensors(crate::sources::CpuSource::get_cached_sensors());
    cpu_config_widget.set_cpu_core_count(crate::sources::CpuSource::get_cached_core_count());

    // Load existing CPU config if source is CPU
    if old_source_id == "cpu" {
        if let Some(cpu_config_value) = panel_guard.config.get("cpu_config") {
            if let Ok(cpu_config) = serde_json::from_value::<crate::ui::CpuSourceConfig>(cpu_config_value.clone()) {
                cpu_config_widget.set_config(cpu_config);
            }
        }
    }

    source_tab_box.append(cpu_config_widget.widget());

    // Wrap cpu_config_widget in Rc for sharing
    let cpu_config_widget = Rc::new(cpu_config_widget);

    // GPU source configuration widget
    let gpu_config_widget = crate::ui::GpuSourceConfigWidget::new();
    gpu_config_widget.widget().set_visible(old_source_id == "gpu");

    // Populate GPU information from cached GPU hardware info
    let gpu_names: Vec<String> = crate::sources::GpuSource::get_cached_gpu_names().to_vec();
    gpu_config_widget.set_available_gpus(&gpu_names);

    // Load existing GPU config if source is GPU
    if old_source_id == "gpu" {
        if let Some(gpu_config_value) = panel_guard.config.get("gpu_config") {
            if let Ok(gpu_config) = serde_json::from_value::<crate::ui::GpuSourceConfig>(gpu_config_value.clone()) {
                gpu_config_widget.set_config(gpu_config);
            }
        }
    }

    source_tab_box.append(gpu_config_widget.widget());

    // Wrap gpu_config_widget in Rc for sharing
    let gpu_config_widget = Rc::new(gpu_config_widget);

    // Memory source configuration widget
    let memory_config_widget = crate::ui::MemorySourceConfigWidget::new();
    memory_config_widget.widget().set_visible(old_source_id == "memory");

    // Load existing Memory config if source is Memory
    if old_source_id == "memory" {
        if let Some(memory_config_value) = panel_guard.config.get("memory_config") {
            if let Ok(memory_config) = serde_json::from_value::<crate::ui::MemorySourceConfig>(memory_config_value.clone()) {
                memory_config_widget.set_config(memory_config);
            }
        }
    }

    source_tab_box.append(memory_config_widget.widget());

    // Wrap memory_config_widget in Rc for sharing
    let memory_config_widget = Rc::new(memory_config_widget);

    // System Temperature source configuration widget
    let system_temp_config_widget = crate::ui::SystemTempConfigWidget::new();
    system_temp_config_widget.widget().set_visible(old_source_id == "system_temp");

    // Load existing System Temp config if source is system_temp
    if old_source_id == "system_temp" {
        if let Some(system_temp_config_value) = panel_guard.config.get("system_temp_config") {
            if let Ok(system_temp_config) = serde_json::from_value::<crate::sources::SystemTempConfig>(system_temp_config_value.clone()) {
                system_temp_config_widget.set_config(system_temp_config);
            }
        }
    }

    source_tab_box.append(system_temp_config_widget.widget());

    // Wrap system_temp_config_widget in Rc for sharing
    let system_temp_config_widget = Rc::new(system_temp_config_widget);

    // Fan Speed source configuration widget
    let fan_speed_config_widget = crate::ui::FanSpeedConfigWidget::new();
    fan_speed_config_widget.widget().set_visible(old_source_id == "fan_speed");

    // Load existing Fan Speed config if source is fan_speed
    if old_source_id == "fan_speed" {
        if let Some(fan_speed_config_value) = panel_guard.config.get("fan_speed_config") {
            if let Ok(fan_speed_config) = serde_json::from_value::<crate::sources::FanSpeedConfig>(fan_speed_config_value.clone()) {
                fan_speed_config_widget.set_config(&fan_speed_config);
            }
        }
    }

    source_tab_box.append(fan_speed_config_widget.widget());

    // Wrap fan_speed_config_widget in Rc for sharing
    let fan_speed_config_widget = Rc::new(fan_speed_config_widget);

    // Disk source configuration widget
    let disk_config_widget = crate::ui::DiskSourceConfigWidget::new();
    disk_config_widget.widget().set_visible(old_source_id == "disk");

    // Populate disk information
    let disks = crate::sources::DiskSource::get_available_disks();
    disk_config_widget.set_available_disks(&disks);

    // Load existing Disk config if source is disk
    if old_source_id == "disk" {
        if let Some(disk_config_value) = panel_guard.config.get("disk_config") {
            if let Ok(disk_config) = serde_json::from_value::<crate::ui::DiskSourceConfig>(disk_config_value.clone()) {
                disk_config_widget.set_config(disk_config);
            }
        }
    }

    source_tab_box.append(disk_config_widget.widget());

    // Wrap disk_config_widget in Rc for sharing
    let disk_config_widget = Rc::new(disk_config_widget);

    // Clock source configuration widget
    let clock_config_widget = crate::ui::ClockSourceConfigWidget::new();
    clock_config_widget.widget().set_visible(old_source_id == "clock");

    // Load existing Clock config if source is clock
    if old_source_id == "clock" {
        if let Some(clock_config_value) = panel_guard.config.get("clock_config") {
            if let Ok(clock_config) = serde_json::from_value::<crate::sources::ClockSourceConfig>(clock_config_value.clone()) {
                clock_config_widget.set_config(&clock_config);
            }
        }
    }

    source_tab_box.append(clock_config_widget.widget());

    // Wrap clock_config_widget in Rc for sharing
    let clock_config_widget = Rc::new(clock_config_widget);

    // === Combination Source Config ===
    let combo_config_widget = crate::ui::ComboSourceConfigWidget::new();
    combo_config_widget.widget().set_visible(old_source_id == "combination");

    // Load existing Combo config if source is combination
    if old_source_id == "combination" {
        if let Some(combo_config_value) = panel_guard.config.get("combo_config") {
            if let Ok(combo_config) = serde_json::from_value::<crate::sources::ComboSourceConfig>(combo_config_value.clone()) {
                combo_config_widget.set_config(combo_config);
            }
        }
    }

    source_tab_box.append(combo_config_widget.widget());

    // Wrap combo_config_widget in Rc<RefCell> for sharing (needs RefCell for set_on_change)
    let combo_config_widget = Rc::new(std::cell::RefCell::new(combo_config_widget));

    // === Test Source Config ===
    let test_config_widget = crate::ui::TestSourceConfigWidget::new();
    test_config_widget.widget().set_visible(old_source_id == "test");

    // Load existing Test config if source is test
    // Priority: saved panel config > global TEST_SOURCE_STATE > defaults
    if old_source_id == "test" {
        let test_config = if let Some(test_config_value) = panel_guard.config.get("test_config") {
            serde_json::from_value::<crate::sources::TestSourceConfig>(test_config_value.clone())
                .unwrap_or_else(|_| {
                    // Fallback to global state if parsing fails
                    crate::sources::TEST_SOURCE_STATE.lock()
                        .map(|state| state.config.clone())
                        .unwrap_or_default()
                })
        } else {
            // No saved config - use current global state to avoid resetting
            crate::sources::TEST_SOURCE_STATE.lock()
                .map(|state| state.config.clone())
                .unwrap_or_default()
        };
        test_config_widget.set_config(&test_config);
    }

    source_tab_box.append(test_config_widget.widget());

    // Wrap test_config_widget in Rc for sharing
    let test_config_widget = Rc::new(test_config_widget);

    // === Static Text Source Config ===
    let static_text_config_widget = crate::ui::StaticTextConfigWidget::new();
    static_text_config_widget.widget().set_visible(old_source_id == "static_text");

    // Load existing Static Text config if source is static_text
    if old_source_id == "static_text" {
        if let Some(static_text_config_value) = panel_guard.config.get("static_text_config") {
            if let Ok(static_text_config) = serde_json::from_value::<crate::sources::StaticTextSourceConfig>(static_text_config_value.clone()) {
                static_text_config_widget.set_config(&static_text_config);
            }
        }
    }

    source_tab_box.append(static_text_config_widget.widget());

    // Wrap static_text_config_widget in Rc for sharing
    let static_text_config_widget = Rc::new(static_text_config_widget);

    // Show/hide source config widgets based on source selection
    {
        let cpu_widget_clone = cpu_config_widget.clone();
        let gpu_widget_clone = gpu_config_widget.clone();
        let memory_widget_clone = memory_config_widget.clone();
        let system_temp_widget_clone = system_temp_config_widget.clone();
        let fan_speed_widget_clone = fan_speed_config_widget.clone();
        let disk_widget_clone = disk_config_widget.clone();
        let clock_widget_clone = clock_config_widget.clone();
        let combo_widget_clone = combo_config_widget.clone();
        let test_widget_clone = test_config_widget.clone();
        let static_text_widget_clone = static_text_config_widget.clone();
        let sources_clone = sources.clone();
        let panel_clone = panel.clone();

        source_combo.connect_selected_notify(move |combo| {
            let selected = combo.selected() as usize;
            if let Some(source_id) = sources_clone.get(selected) {
                cpu_widget_clone.widget().set_visible(source_id == "cpu");
                gpu_widget_clone.widget().set_visible(source_id == "gpu");
                memory_widget_clone.widget().set_visible(source_id == "memory");
                system_temp_widget_clone.widget().set_visible(source_id == "system_temp");
                fan_speed_widget_clone.widget().set_visible(source_id == "fan_speed");
                disk_widget_clone.widget().set_visible(source_id == "disk");
                clock_widget_clone.widget().set_visible(source_id == "clock");
                combo_widget_clone.borrow().widget().set_visible(source_id == "combination");
                test_widget_clone.widget().set_visible(source_id == "test");
                static_text_widget_clone.widget().set_visible(source_id == "static_text");

                // Reload config for the selected source
                {
                    let panel_guard = panel_clone.blocking_read();
                    match source_id.as_str() {
                        "cpu" => {
                            if let Some(cpu_config_value) = panel_guard.config.get("cpu_config") {
                                if let Ok(cpu_config) = serde_json::from_value::<crate::ui::CpuSourceConfig>(cpu_config_value.clone()) {
                                    cpu_widget_clone.set_config(cpu_config);
                                }
                            }
                        }
                        "gpu" => {
                            if let Some(gpu_config_value) = panel_guard.config.get("gpu_config") {
                                if let Ok(gpu_config) = serde_json::from_value::<crate::ui::GpuSourceConfig>(gpu_config_value.clone()) {
                                    gpu_widget_clone.set_config(gpu_config);
                                }
                            }
                        }
                        "memory" => {
                            if let Some(memory_config_value) = panel_guard.config.get("memory_config") {
                                if let Ok(memory_config) = serde_json::from_value::<crate::ui::MemorySourceConfig>(memory_config_value.clone()) {
                                    memory_widget_clone.set_config(memory_config);
                                }
                            }
                        }
                        "system_temp" => {
                            if let Some(system_temp_config_value) = panel_guard.config.get("system_temp_config") {
                                if let Ok(system_temp_config) = serde_json::from_value::<crate::sources::SystemTempConfig>(system_temp_config_value.clone()) {
                                    system_temp_widget_clone.set_config(system_temp_config);
                                }
                            }
                        }
                        "fan_speed" => {
                            if let Some(fan_speed_config_value) = panel_guard.config.get("fan_speed_config") {
                                if let Ok(fan_speed_config) = serde_json::from_value::<crate::sources::FanSpeedConfig>(fan_speed_config_value.clone()) {
                                    fan_speed_widget_clone.set_config(&fan_speed_config);
                                }
                            }
                        }
                        "disk" => {
                            if let Some(disk_config_value) = panel_guard.config.get("disk_config") {
                                if let Ok(disk_config) = serde_json::from_value::<crate::ui::DiskSourceConfig>(disk_config_value.clone()) {
                                    disk_widget_clone.set_config(disk_config);
                                }
                            }
                        }
                        "clock" => {
                            if let Some(clock_config_value) = panel_guard.config.get("clock_config") {
                                if let Ok(clock_config) = serde_json::from_value::<crate::sources::ClockSourceConfig>(clock_config_value.clone()) {
                                    clock_widget_clone.set_config(&clock_config);
                                }
                            }
                        }
                        "combination" => {
                            if let Some(combo_config_value) = panel_guard.config.get("combo_config") {
                                if let Ok(combo_config) = serde_json::from_value::<crate::sources::ComboSourceConfig>(combo_config_value.clone()) {
                                    combo_widget_clone.borrow().set_config(combo_config);
                                }
                            }
                        }
                        "test" => {
                            // Load existing config or use current global state to avoid resetting
                            let test_config = if let Some(test_config_value) = panel_guard.config.get("test_config") {
                                serde_json::from_value::<crate::sources::TestSourceConfig>(test_config_value.clone())
                                    .unwrap_or_else(|_| {
                                        crate::sources::TEST_SOURCE_STATE.lock()
                                            .map(|state| state.config.clone())
                                            .unwrap_or_default()
                                    })
                            } else {
                                // No saved config - use current global state
                                crate::sources::TEST_SOURCE_STATE.lock()
                                    .map(|state| state.config.clone())
                                    .unwrap_or_default()
                            };
                            test_widget_clone.set_config(&test_config);
                        }
                        "static_text" => {
                            if let Some(static_text_config_value) = panel_guard.config.get("static_text_config") {
                                if let Ok(static_text_config) = serde_json::from_value::<crate::sources::StaticTextSourceConfig>(static_text_config_value.clone()) {
                                    static_text_widget_clone.set_config(&static_text_config);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
    }

    notebook.append_page(&source_tab_box, Some(&Label::new(Some("Data Source"))));

    // === Tab 3: Display Type ===
    let displayer_tab_box = GtkBox::new(Orientation::Vertical, 12);
    displayer_tab_box.set_margin_top(12);
    displayer_tab_box.set_margin_bottom(12);
    displayer_tab_box.set_margin_start(12);
    displayer_tab_box.set_margin_end(12);

    let displayer_label = Label::new(Some("Display Type"));
    displayer_label.add_css_class("heading");
    displayer_tab_box.append(&displayer_label);

    let displayer_box = GtkBox::new(Orientation::Horizontal, 6);
    displayer_box.set_margin_start(12);

    let displayer_combo_label = Label::new(Some("Displayer:"));

    // Populate displayer dropdown
    let displayers = registry.list_displayers();
    let mut selected_displayer_idx = 0;
    for (idx, displayer_id) in displayers.iter().enumerate() {
        if displayer_id == &old_displayer_id {
            selected_displayer_idx = idx;
        }
    }

    let displayer_strings: Vec<&str> = displayers.iter().map(|s| s.as_str()).collect();
    let displayer_list = StringList::new(&displayer_strings);
    let displayer_combo = DropDown::new(Some(displayer_list), Option::<gtk4::Expression>::None);
    displayer_combo.set_selected(selected_displayer_idx as u32);

    displayer_box.append(&displayer_combo_label);
    displayer_box.append(&displayer_combo);
    displayer_tab_box.append(&displayer_box);

    // Text displayer configuration (shown only when text displayer is selected)
    let text_config_label = Label::new(Some("Text Configuration"));
    text_config_label.add_css_class("heading");
    text_config_label.set_margin_top(12);

    // Get available fields from the current data source
    let available_fields = panel_guard.source.fields();

    let text_config_widget = crate::ui::TextLineConfigWidget::new(available_fields.clone());
    text_config_widget.widget().set_visible(old_displayer_id == "text");
    text_config_label.set_visible(old_displayer_id == "text");

    // Load existing text config if displayer is text
    // Prefer getting config directly from displayer (most up-to-date), fall back to panel config
    if old_displayer_id == "text" {
        let config_loaded = if let Some(crate::core::DisplayerConfig::Text(text_config)) = panel_guard.displayer.get_typed_config() {
            text_config_widget.set_config(text_config);
            true
        } else {
            false
        };

        // Fall back to panel config hashmap if get_typed_config didn't work
        if !config_loaded {
            let text_config = if let Some(lines_value) = panel_guard.config.get("lines") {
                // Load from saved config
                serde_json::from_value::<crate::displayers::TextDisplayerConfig>(
                    serde_json::json!({ "lines": lines_value })
                ).unwrap_or_default()
            } else {
                // Use default config if no saved config exists
                crate::displayers::TextDisplayerConfig::default()
            };
            text_config_widget.set_config(text_config);
        }
    }

    displayer_tab_box.append(&text_config_label);
    displayer_tab_box.append(text_config_widget.widget());

    // Wrap text_config_widget in Rc for sharing
    let text_config_widget = Rc::new(text_config_widget);

    // Bar displayer configuration (shown only when bar displayer is selected)
    let bar_config_label = Label::new(Some("Bar Configuration"));
    bar_config_label.add_css_class("heading");
    bar_config_label.set_margin_top(12);

    // Get available fields from the current data source (same as text displayer)
    let bar_config_widget = crate::ui::BarConfigWidget::new(available_fields.clone());
    bar_config_widget.widget().set_visible(old_displayer_id == "bar");
    bar_config_label.set_visible(old_displayer_id == "bar");

    // Load existing bar config if displayer is bar, or use default
    if old_displayer_id == "bar" {
        let bar_config = if let Some(bar_config_value) = panel_guard.config.get("bar_config") {
            // Use saved config if available
            serde_json::from_value::<crate::ui::BarDisplayConfig>(bar_config_value.clone())
                .unwrap_or_else(|_| crate::ui::BarDisplayConfig::default())
        } else {
            // Use default config (includes caption, value, unit text lines)
            crate::ui::BarDisplayConfig::default()
        };
        bar_config_widget.set_config(bar_config);
    }

    displayer_tab_box.append(&bar_config_label);
    displayer_tab_box.append(bar_config_widget.widget());

    // Wrap bar_config_widget in Rc for sharing
    let bar_config_widget = Rc::new(bar_config_widget);

    // Arc displayer configuration (shown only when arc displayer is selected)
    let arc_config_label = Label::new(Some("Arc Gauge Configuration"));
    arc_config_label.add_css_class("heading");
    arc_config_label.set_margin_top(12);

    let arc_config_widget = crate::ui::ArcConfigWidget::new(available_fields.clone());
    arc_config_widget.widget().set_visible(old_displayer_id == "arc");
    arc_config_label.set_visible(old_displayer_id == "arc");

    // Load existing arc config if displayer is arc, or use default
    if old_displayer_id == "arc" {
        let arc_config = if let Some(arc_config_value) = panel_guard.config.get("arc_config") {
            // Use saved config if available
            serde_json::from_value::<crate::ui::ArcDisplayConfig>(arc_config_value.clone())
                .unwrap_or_else(|_| crate::ui::ArcDisplayConfig::default())
        } else {
            // Use default config
            crate::ui::ArcDisplayConfig::default()
        };
        arc_config_widget.set_config(arc_config);
    }

    displayer_tab_box.append(&arc_config_label);
    displayer_tab_box.append(arc_config_widget.widget());

    // Wrap arc_config_widget in Rc for sharing
    let arc_config_widget = Rc::new(arc_config_widget);

    // Speedometer displayer configuration (shown only when speedometer displayer is selected)
    let speedometer_config_label = Label::new(Some("Speedometer Gauge Configuration"));
    speedometer_config_label.add_css_class("heading");
    speedometer_config_label.set_margin_top(12);

    let speedometer_config_widget = crate::ui::SpeedometerConfigWidget::new(available_fields.clone());
    speedometer_config_widget.widget().set_visible(old_displayer_id == "speedometer");
    speedometer_config_label.set_visible(old_displayer_id == "speedometer");

    // Load existing speedometer config if displayer is speedometer, or use default
    if old_displayer_id == "speedometer" {
        let speedometer_config = if let Some(speedometer_config_value) = panel_guard.config.get("speedometer_config") {
            // Use saved config if available
            serde_json::from_value::<crate::ui::SpeedometerConfig>(speedometer_config_value.clone())
                .unwrap_or_else(|_| crate::ui::SpeedometerConfig::default())
        } else {
            // Use default config
            crate::ui::SpeedometerConfig::default()
        };
        speedometer_config_widget.set_config(&speedometer_config);
    }

    displayer_tab_box.append(&speedometer_config_label);
    displayer_tab_box.append(speedometer_config_widget.widget());

    // Wrap speedometer_config_widget in Rc for sharing
    let speedometer_config_widget = Rc::new(speedometer_config_widget);

    // Graph displayer configuration widget
    let graph_config_label = Label::new(Some("Graph Configuration:"));
    graph_config_label.set_halign(gtk4::Align::Start);
    graph_config_label.add_css_class("heading");
    graph_config_label.set_visible(old_displayer_id == "graph");

    let graph_config_widget = crate::ui::GraphConfigWidget::new(available_fields.clone());
    graph_config_widget.widget().set_visible(old_displayer_id == "graph");

    // Load existing graph config if displayer is graph, or use default
    if old_displayer_id == "graph" {
        let graph_config = if let Some(graph_config_value) = panel_guard.config.get("graph_config") {
            // Use saved config if available
            serde_json::from_value::<crate::ui::GraphDisplayConfig>(graph_config_value.clone())
                .unwrap_or_else(|_| crate::ui::GraphDisplayConfig::default())
        } else {
            // Use default config
            crate::ui::GraphDisplayConfig::default()
        };
        graph_config_widget.set_config(graph_config);
    }

    displayer_tab_box.append(&graph_config_label);
    displayer_tab_box.append(graph_config_widget.widget());

    // Wrap graph_config_widget in Rc for sharing
    let graph_config_widget = Rc::new(graph_config_widget);

    // Analog Clock displayer configuration widget
    let clock_analog_config_label = Label::new(Some("Analog Clock Configuration:"));
    clock_analog_config_label.set_halign(gtk4::Align::Start);
    clock_analog_config_label.add_css_class("heading");
    clock_analog_config_label.set_visible(old_displayer_id == "clock_analog");

    let clock_analog_config_widget = crate::ui::ClockAnalogConfigWidget::new();
    clock_analog_config_widget.widget().set_visible(old_displayer_id == "clock_analog");

    // Load existing analog clock config if displayer is clock_analog
    if old_displayer_id == "clock_analog" {
        // Try new key first, then legacy key for backwards compatibility
        let config_value = panel_guard.config.get("clock_analog_config")
            .or_else(|| panel_guard.config.get("analog_clock_config"));
        if let Some(config_value) = config_value {
            if let Ok(config) = serde_json::from_value::<crate::ui::AnalogClockConfig>(config_value.clone()) {
                clock_analog_config_widget.set_config(config);
            }
        }
    }

    displayer_tab_box.append(&clock_analog_config_label);
    displayer_tab_box.append(clock_analog_config_widget.widget());

    // Wrap clock_analog_config_widget in Rc for sharing
    let clock_analog_config_widget = Rc::new(clock_analog_config_widget);

    // Digital Clock displayer configuration widget
    let clock_digital_config_label = Label::new(Some("Digital Clock Configuration:"));
    clock_digital_config_label.set_halign(gtk4::Align::Start);
    clock_digital_config_label.add_css_class("heading");
    clock_digital_config_label.set_visible(old_displayer_id == "clock_digital");

    let clock_digital_config_widget = crate::ui::ClockDigitalConfigWidget::new();
    clock_digital_config_widget.widget().set_visible(old_displayer_id == "clock_digital");

    // Load existing digital clock config if displayer is clock_digital
    if old_displayer_id == "clock_digital" {
        // Try new key first, then legacy key for backwards compatibility
        let config_value = panel_guard.config.get("clock_digital_config")
            .or_else(|| panel_guard.config.get("digital_clock_config"));
        if let Some(config_value) = config_value {
            if let Ok(config) = serde_json::from_value::<crate::displayers::DigitalClockConfig>(config_value.clone()) {
                clock_digital_config_widget.set_config(config);
            }
        }
    }

    displayer_tab_box.append(&clock_digital_config_label);
    displayer_tab_box.append(clock_digital_config_widget.widget());

    // Wrap clock_digital_config_widget in Rc for sharing
    let clock_digital_config_widget = Rc::new(clock_digital_config_widget);

    // === LCARS Configuration (Lazy Initialization) ===
    let lcars_config_label = Label::new(Some("LCARS Configuration:"));
    lcars_config_label.set_halign(gtk4::Align::Start);
    lcars_config_label.add_css_class("heading");
    lcars_config_label.set_visible(old_displayer_id == "lcars");

    // Create placeholder box for lazy widget creation
    let lcars_placeholder = GtkBox::new(Orientation::Vertical, 0);
    lcars_placeholder.set_visible(old_displayer_id == "lcars");

    // Use Option for lazy initialization - only create widget when needed
    let lcars_config_widget: Rc<RefCell<Option<crate::ui::LcarsConfigWidget>>> = Rc::new(RefCell::new(None));

    // Only create LCARS widget if this is the active displayer (lazy init)
    if old_displayer_id == "lcars" {
        log::info!("=== Creating LcarsConfigWidget (lazy init), old_displayer='{}', old_source='{}' ===", old_displayer_id, old_source_id);
        let widget = crate::ui::LcarsConfigWidget::new(available_fields.clone());

        // Load existing LCARS config
        let config_loaded = if let Some(crate::core::DisplayerConfig::Lcars(lcars_config)) = panel_guard.displayer.get_typed_config() {
            log::info!("=== Loading LCARS config from displayer.get_typed_config() ===");
            widget.set_config(lcars_config);
            true
        } else {
            false
        };

        if !config_loaded {
            if let Some(config_value) = panel_guard.config.get("lcars_config") {
                if let Ok(config) = serde_json::from_value::<crate::displayers::LcarsDisplayConfig>(config_value.clone()) {
                    log::info!("=== Loading LCARS config from panel config hashmap ===");
                    widget.set_config(config);
                }
            }
        }

        lcars_placeholder.append(widget.widget());
        *lcars_config_widget.borrow_mut() = Some(widget);
    }

    displayer_tab_box.append(&lcars_config_label);
    displayer_tab_box.append(&lcars_placeholder);

    // === CPU Cores Configuration ===
    let cpu_cores_config_label = Label::new(Some("CPU Cores Configuration:"));
    cpu_cores_config_label.set_halign(gtk4::Align::Start);
    cpu_cores_config_label.add_css_class("heading");
    cpu_cores_config_label.set_visible(old_displayer_id == "cpu_cores");

    let cpu_cores_config_widget = crate::ui::CoreBarsConfigWidget::new();
    cpu_cores_config_widget.widget().set_visible(old_displayer_id == "cpu_cores");

    // Load existing CPU cores config if displayer is cpu_cores
    if old_displayer_id == "cpu_cores" {
        if let Some(config_value) = panel_guard.config.get("core_bars_config") {
            if let Ok(config) = serde_json::from_value::<crate::ui::CoreBarsConfig>(config_value.clone()) {
                cpu_cores_config_widget.set_config(config);
            }
        }
    }

    // Count available CPU cores from source fields (e.g., "core0_usage", "core1_usage", ...)
    let core_count = available_fields.iter()
        .filter(|f| f.id.starts_with("core") && f.id.ends_with("_usage"))
        .count();
    if core_count > 0 {
        cpu_cores_config_widget.set_max_cores(core_count);
    }

    displayer_tab_box.append(&cpu_cores_config_label);
    displayer_tab_box.append(cpu_cores_config_widget.widget());

    // Set up change callback so the internal preview updates
    cpu_cores_config_widget.set_on_change(|| {});

    // Wrap cpu_cores_config_widget in Rc for sharing
    let cpu_cores_config_widget = Rc::new(cpu_cores_config_widget);

    // === Indicator Configuration ===
    let indicator_config_label = Label::new(Some("Indicator Configuration:"));
    indicator_config_label.set_halign(gtk4::Align::Start);
    indicator_config_label.add_css_class("heading");
    indicator_config_label.set_visible(old_displayer_id == "indicator");

    let indicator_config_widget = crate::ui::IndicatorConfigWidget::new(available_fields.clone());
    indicator_config_widget.widget().set_visible(old_displayer_id == "indicator");

    // Load existing Indicator config if displayer is indicator
    if old_displayer_id == "indicator" {
        if let Some(config_value) = panel_guard.config.get("indicator_config") {
            if let Ok(config) = serde_json::from_value::<crate::displayers::IndicatorConfig>(config_value.clone()) {
                indicator_config_widget.set_config(&config);
            }
        }
    }

    displayer_tab_box.append(&indicator_config_label);
    displayer_tab_box.append(indicator_config_widget.widget());

    // Set up change callback
    indicator_config_widget.set_on_change(|| {});

    // Wrap indicator_config_widget in Rc for sharing
    let indicator_config_widget = Rc::new(indicator_config_widget);

    // === Cyberpunk Configuration (Lazy Initialization) ===
    let cyberpunk_config_label = Label::new(Some("Cyberpunk HUD Configuration:"));
    cyberpunk_config_label.set_halign(gtk4::Align::Start);
    cyberpunk_config_label.add_css_class("heading");
    cyberpunk_config_label.set_visible(old_displayer_id == "cyberpunk");

    // Create placeholder box for lazy widget creation
    let cyberpunk_placeholder = GtkBox::new(Orientation::Vertical, 0);
    cyberpunk_placeholder.set_visible(old_displayer_id == "cyberpunk");

    // Use Option for lazy initialization - only create widget when needed
    let cyberpunk_config_widget: Rc<RefCell<Option<crate::ui::CyberpunkConfigWidget>>> = Rc::new(RefCell::new(None));

    // Only create Cyberpunk widget if this is the active displayer (lazy init)
    if old_displayer_id == "cyberpunk" {
        log::info!("=== Creating CyberpunkConfigWidget (lazy init) ===");
        let widget = crate::ui::CyberpunkConfigWidget::new(available_fields.clone());

        // Load existing Cyberpunk config
        let config_loaded = if let Some(crate::core::DisplayerConfig::Cyberpunk(cyberpunk_config)) = panel_guard.displayer.get_typed_config() {
            log::info!("=== Loading Cyberpunk config from displayer.get_typed_config() ===");
            widget.set_config(&cyberpunk_config);
            true
        } else {
            false
        };

        if !config_loaded {
            if let Some(config_value) = panel_guard.config.get("cyberpunk_config") {
                if let Ok(config) = serde_json::from_value::<crate::displayers::CyberpunkDisplayConfig>(config_value.clone()) {
                    log::info!("=== Loading Cyberpunk config from panel config hashmap ===");
                    widget.set_config(&config);
                }
            }
        }

        widget.set_on_change(|| {});
        cyberpunk_placeholder.append(widget.widget());
        *cyberpunk_config_widget.borrow_mut() = Some(widget);
    }

    displayer_tab_box.append(&cyberpunk_config_label);
    displayer_tab_box.append(&cyberpunk_placeholder);

    // === Material Cards Configuration (Lazy Initialization) ===
    let material_config_label = Label::new(Some("Material Cards Configuration:"));
    material_config_label.set_halign(gtk4::Align::Start);
    material_config_label.add_css_class("heading");
    material_config_label.set_visible(old_displayer_id == "material");

    // Create placeholder box for lazy widget creation
    let material_placeholder = GtkBox::new(Orientation::Vertical, 0);
    material_placeholder.set_visible(old_displayer_id == "material");

    // Use Option for lazy initialization - only create widget when needed
    let material_config_widget: Rc<RefCell<Option<crate::ui::MaterialConfigWidget>>> = Rc::new(RefCell::new(None));

    // Only create Material widget if this is the active displayer (lazy init)
    if old_displayer_id == "material" {
        log::info!("=== Creating MaterialConfigWidget (lazy init) ===");
        let widget = crate::ui::MaterialConfigWidget::new(available_fields.clone());

        // Load existing Material config
        let config_loaded = if let Some(crate::core::DisplayerConfig::Material(material_config)) = panel_guard.displayer.get_typed_config() {
            log::info!("=== Loading Material config from displayer.get_typed_config() ===");
            widget.set_config(&material_config);
            true
        } else {
            false
        };

        if !config_loaded {
            if let Some(config_value) = panel_guard.config.get("material_config") {
                if let Ok(config) = serde_json::from_value::<crate::displayers::MaterialDisplayConfig>(config_value.clone()) {
                    log::info!("=== Loading Material config from panel config hashmap ===");
                    widget.set_config(&config);
                }
            }
        }

        widget.set_on_change(|| {});
        material_placeholder.append(widget.widget());
        *material_config_widget.borrow_mut() = Some(widget);
    }

    displayer_tab_box.append(&material_config_label);
    displayer_tab_box.append(&material_placeholder);

    // === Industrial Gauge Configuration (Lazy Initialization) ===
    let industrial_config_label = Label::new(Some("Industrial Gauge Configuration:"));
    industrial_config_label.set_halign(gtk4::Align::Start);
    industrial_config_label.add_css_class("heading");
    industrial_config_label.set_visible(old_displayer_id == "industrial");

    // Create placeholder box for lazy widget creation
    let industrial_placeholder = GtkBox::new(Orientation::Vertical, 0);
    industrial_placeholder.set_visible(old_displayer_id == "industrial");

    // Use Option for lazy initialization - only create widget when needed
    let industrial_config_widget: Rc<RefCell<Option<crate::ui::IndustrialConfigWidget>>> = Rc::new(RefCell::new(None));

    // Only create Industrial widget if this is the active displayer (lazy init)
    if old_displayer_id == "industrial" {
        log::info!("=== Creating IndustrialConfigWidget (lazy init) ===");
        let widget = crate::ui::IndustrialConfigWidget::new(available_fields.clone());

        // Load existing Industrial config
        let config_loaded = if let Some(crate::core::DisplayerConfig::Industrial(industrial_config)) = panel_guard.displayer.get_typed_config() {
            log::info!("=== Loading Industrial config from displayer.get_typed_config() ===");
            widget.set_config(&industrial_config);
            true
        } else {
            false
        };

        if !config_loaded {
            if let Some(config_value) = panel_guard.config.get("industrial_config") {
                if let Ok(config) = serde_json::from_value::<crate::displayers::IndustrialDisplayConfig>(config_value.clone()) {
                    log::info!("=== Loading Industrial config from panel config hashmap ===");
                    widget.set_config(&config);
                }
            }
        }

        widget.set_on_change(|| {});
        industrial_placeholder.append(widget.widget());
        *industrial_config_widget.borrow_mut() = Some(widget);
    }

    displayer_tab_box.append(&industrial_config_label);
    displayer_tab_box.append(&industrial_placeholder);

    // === Retro Terminal Configuration (Lazy Initialization) ===
    let retro_terminal_config_label = Label::new(Some("Retro Terminal Configuration:"));
    retro_terminal_config_label.set_halign(gtk4::Align::Start);
    retro_terminal_config_label.add_css_class("heading");
    retro_terminal_config_label.set_visible(old_displayer_id == "retro_terminal");

    // Create placeholder box for lazy widget creation
    let retro_terminal_placeholder = GtkBox::new(Orientation::Vertical, 0);
    retro_terminal_placeholder.set_visible(old_displayer_id == "retro_terminal");

    // Use Option for lazy initialization - only create widget when needed
    let retro_terminal_config_widget: Rc<RefCell<Option<crate::ui::RetroTerminalConfigWidget>>> = Rc::new(RefCell::new(None));

    // Only create Retro Terminal widget if this is the active displayer (lazy init)
    if old_displayer_id == "retro_terminal" {
        log::info!("=== Creating RetroTerminalConfigWidget (lazy init) ===");
        let widget = crate::ui::RetroTerminalConfigWidget::new(available_fields.clone());

        // Load existing Retro Terminal config
        let config_loaded = if let Some(crate::core::DisplayerConfig::RetroTerminal(retro_config)) = panel_guard.displayer.get_typed_config() {
            log::info!("=== Loading Retro Terminal config from displayer.get_typed_config() ===");
            widget.set_config(&retro_config);
            true
        } else {
            false
        };

        if !config_loaded {
            if let Some(config_value) = panel_guard.config.get("retro_terminal_config") {
                if let Ok(config) = serde_json::from_value::<crate::displayers::RetroTerminalDisplayConfig>(config_value.clone()) {
                    log::info!("=== Loading Retro Terminal config from panel config hashmap ===");
                    widget.set_config(&config);
                }
            }
        }

        widget.set_on_change(|| {});
        retro_terminal_placeholder.append(widget.widget());
        *retro_terminal_config_widget.borrow_mut() = Some(widget);
    }

    displayer_tab_box.append(&retro_terminal_config_label);
    displayer_tab_box.append(&retro_terminal_placeholder);

    // === Fighter HUD Configuration (Lazy Initialization) ===
    let fighter_hud_config_label = Label::new(Some("Fighter HUD Configuration:"));
    fighter_hud_config_label.set_halign(gtk4::Align::Start);
    fighter_hud_config_label.add_css_class("heading");
    fighter_hud_config_label.set_visible(old_displayer_id == "fighter_hud");

    // Create placeholder box for lazy widget creation
    let fighter_hud_placeholder = GtkBox::new(Orientation::Vertical, 0);
    fighter_hud_placeholder.set_visible(old_displayer_id == "fighter_hud");

    // Use Option for lazy initialization - only create widget when needed
    let fighter_hud_config_widget: Rc<RefCell<Option<crate::ui::FighterHudConfigWidget>>> = Rc::new(RefCell::new(None));

    // Only create Fighter HUD widget if this is the active displayer (lazy init)
    if old_displayer_id == "fighter_hud" {
        log::info!("=== Creating FighterHudConfigWidget (lazy init) ===");
        let widget = crate::ui::FighterHudConfigWidget::new(available_fields.clone());

        // Load existing Fighter HUD config
        let config_loaded = if let Some(crate::core::DisplayerConfig::FighterHud(hud_config)) = panel_guard.displayer.get_typed_config() {
            log::info!("=== Loading Fighter HUD config from displayer.get_typed_config() ===");
            widget.set_config(hud_config);
            true
        } else {
            false
        };

        if !config_loaded {
            if let Some(config_value) = panel_guard.config.get("fighter_hud_config") {
                if let Ok(config) = serde_json::from_value::<crate::displayers::FighterHudDisplayConfig>(config_value.clone()) {
                    log::info!("=== Loading Fighter HUD config from panel config hashmap ===");
                    widget.set_config(config);
                }
            }
        }

        widget.set_on_change(|| {});
        fighter_hud_placeholder.append(widget.widget());
        *fighter_hud_config_widget.borrow_mut() = Some(widget);
    }

    displayer_tab_box.append(&fighter_hud_config_label);
    displayer_tab_box.append(&fighter_hud_placeholder);

    // === Synthwave Configuration (Lazy Initialization) ===
    let synthwave_config_label = Label::new(Some("Synthwave Configuration:"));
    synthwave_config_label.set_halign(gtk4::Align::Start);
    synthwave_config_label.add_css_class("heading");
    synthwave_config_label.set_visible(old_displayer_id == "synthwave");

    // Create placeholder box for lazy widget creation
    let synthwave_placeholder = GtkBox::new(Orientation::Vertical, 0);
    synthwave_placeholder.set_visible(old_displayer_id == "synthwave");

    // Use Option for lazy initialization - only create widget when needed
    let synthwave_config_widget: Rc<RefCell<Option<crate::ui::SynthwaveConfigWidget>>> = Rc::new(RefCell::new(None));

    // Only create Synthwave widget if this is the active displayer (lazy init)
    if old_displayer_id == "synthwave" {
        log::info!("=== Creating SynthwaveConfigWidget (lazy init) ===");
        let widget = crate::ui::SynthwaveConfigWidget::new(available_fields.clone());

        // Load existing Synthwave config
        let config_loaded = if let Some(crate::core::DisplayerConfig::Synthwave(sw_config)) = panel_guard.displayer.get_typed_config() {
            log::info!("=== Loading Synthwave config from displayer.get_typed_config() ===");
            widget.set_config(sw_config);
            true
        } else {
            false
        };

        if !config_loaded {
            if let Some(config_value) = panel_guard.config.get("synthwave_config") {
                if let Ok(config) = serde_json::from_value::<crate::displayers::SynthwaveDisplayConfig>(config_value.clone()) {
                    log::info!("=== Loading Synthwave config from panel config hashmap ===");
                    widget.set_config(config);
                }
            }
        }

        widget.set_on_change(|| {});
        synthwave_placeholder.append(widget.widget());
        *synthwave_config_widget.borrow_mut() = Some(widget);
    }

    displayer_tab_box.append(&synthwave_config_label);
    displayer_tab_box.append(&synthwave_placeholder);

    // Connect combo_config_widget to update ONLY the active displayer's config widget when sources change
    // Other widgets are updated lazily when the user switches to them (see displayer_combo handlers below)
    {
        let lcars_widget_clone = lcars_config_widget.clone();
        let cyberpunk_widget_clone = cyberpunk_config_widget.clone();
        let material_widget_clone = material_config_widget.clone();
        let industrial_widget_clone = industrial_config_widget.clone();
        let retro_terminal_widget_clone = retro_terminal_config_widget.clone();
        let fighter_hud_widget_clone = fighter_hud_config_widget.clone();
        let synthwave_widget_clone = synthwave_config_widget.clone();
        let combo_widget_for_lcars = combo_config_widget.clone();
        let panel_for_combo_change = panel.clone();
        combo_config_widget.borrow_mut().set_on_change(move || {
            // Get source summaries from combo config
            let widget = combo_widget_for_lcars.borrow();
            let summaries = widget.get_source_summaries();
            let fields = widget.get_available_fields();
            drop(widget);

            // Only update the ACTIVE displayer's config widget to avoid expensive rebuilds
            // Other widgets will be updated when the user switches to them
            if let Ok(mut panel_guard) = panel_for_combo_change.try_write() {
                let displayer_id = panel_guard.displayer.id().to_string();

                // Update and apply config for the active displayer only (if widget exists)
                match displayer_id.as_str() {
                    "industrial" => {
                        if let Some(ref widget) = *industrial_widget_clone.borrow() {
                            widget.set_available_fields(fields);
                            widget.set_source_summaries(summaries);
                            let config = widget.get_config();
                            if let Ok(config_json) = serde_json::to_value(&config) {
                                panel_guard.config.insert("industrial_config".to_string(), config_json);
                                let config_clone = panel_guard.config.clone();
                                if let Err(e) = panel_guard.apply_config(config_clone) {
                                    log::warn!("Failed to apply Industrial config on source change: {}", e);
                                }
                            }
                        }
                    }
                    "lcars" => {
                        if let Some(ref widget) = *lcars_widget_clone.borrow() {
                            widget.set_available_fields(fields);
                            widget.set_source_summaries(summaries);
                            let config = widget.get_config();
                            if let Ok(config_json) = serde_json::to_value(&config) {
                                panel_guard.config.insert("lcars_config".to_string(), config_json);
                                let config_clone = panel_guard.config.clone();
                                if let Err(e) = panel_guard.apply_config(config_clone) {
                                    log::warn!("Failed to apply LCARS config on source change: {}", e);
                                }
                            }
                        }
                    }
                    "cyberpunk" => {
                        if let Some(ref widget) = *cyberpunk_widget_clone.borrow() {
                            widget.set_available_fields(fields);
                            widget.set_source_summaries(summaries);
                            let config = widget.get_config();
                            if let Ok(config_json) = serde_json::to_value(&config) {
                                panel_guard.config.insert("cyberpunk_config".to_string(), config_json);
                                let config_clone = panel_guard.config.clone();
                                if let Err(e) = panel_guard.apply_config(config_clone) {
                                    log::warn!("Failed to apply Cyberpunk config on source change: {}", e);
                                }
                            }
                        }
                    }
                    "material" => {
                        if let Some(ref widget) = *material_widget_clone.borrow() {
                            widget.set_available_fields(fields);
                            widget.set_source_summaries(summaries);
                            let config = widget.get_config();
                            if let Ok(config_json) = serde_json::to_value(&config) {
                                panel_guard.config.insert("material_config".to_string(), config_json);
                                let config_clone = panel_guard.config.clone();
                                if let Err(e) = panel_guard.apply_config(config_clone) {
                                    log::warn!("Failed to apply Material config on source change: {}", e);
                                }
                            }
                        }
                    }
                    "retro_terminal" => {
                        if let Some(ref widget) = *retro_terminal_widget_clone.borrow() {
                            widget.set_available_fields(fields);
                            widget.set_source_summaries(summaries);
                            let config = widget.get_config();
                            if let Ok(config_json) = serde_json::to_value(&config) {
                                panel_guard.config.insert("retro_terminal_config".to_string(), config_json);
                                let config_clone = panel_guard.config.clone();
                                if let Err(e) = panel_guard.apply_config(config_clone) {
                                    log::warn!("Failed to apply Retro Terminal config on source change: {}", e);
                                }
                            }
                        }
                    }
                    "fighter_hud" => {
                        if let Some(ref widget) = *fighter_hud_widget_clone.borrow() {
                            widget.set_available_fields(fields);
                            widget.set_source_summaries(summaries);
                            let config = widget.get_config();
                            if let Ok(config_json) = serde_json::to_value(&config) {
                                panel_guard.config.insert("fighter_hud_config".to_string(), config_json);
                                let config_clone = panel_guard.config.clone();
                                if let Err(e) = panel_guard.apply_config(config_clone) {
                                    log::warn!("Failed to apply Fighter HUD config on source change: {}", e);
                                }
                            }
                        }
                    }
                    "synthwave" => {
                        if let Some(ref widget) = *synthwave_widget_clone.borrow() {
                            widget.set_available_fields(fields);
                            widget.set_source_summaries(summaries);
                            let config = widget.get_config();
                            if let Ok(config_json) = serde_json::to_value(&config) {
                                panel_guard.config.insert("synthwave_config".to_string(), config_json);
                                let config_clone = panel_guard.config.clone();
                                if let Err(e) = panel_guard.apply_config(config_clone) {
                                    log::warn!("Failed to apply Synthwave config on source change: {}", e);
                                }
                            }
                        }
                    }
                    _ => {
                        // For non-combo displayers, no update needed
                    }
                }
            }
        });

        // Initialize the ACTIVE combo config widget with current source summaries if combo source is selected
        // (Other widgets are created lazily and will be initialized when switched to)
        if old_source_id == "combination" {
            let widget = combo_config_widget.borrow();
            let summaries = widget.get_source_summaries();
            let fields = widget.get_available_fields();
            drop(widget);
            log::info!("=== Initializing active combo widget at startup: {} summaries, {} fields ===", summaries.len(), fields.len());

            // Only initialize the widget that exists (the active one)
            if let Some(ref widget) = *lcars_config_widget.borrow() {
                widget.set_available_fields(fields.clone());
                widget.set_source_summaries(summaries.clone());
            }
            if let Some(ref widget) = *cyberpunk_config_widget.borrow() {
                widget.set_available_fields(fields.clone());
                widget.set_source_summaries(summaries.clone());
            }
            if let Some(ref widget) = *material_config_widget.borrow() {
                widget.set_available_fields(fields.clone());
                widget.set_source_summaries(summaries.clone());
            }
            if let Some(ref widget) = *industrial_config_widget.borrow() {
                widget.set_available_fields(fields.clone());
                widget.set_source_summaries(summaries.clone());
            }
            if let Some(ref widget) = *retro_terminal_config_widget.borrow() {
                widget.set_available_fields(fields.clone());
                widget.set_source_summaries(summaries.clone());
            }
            if let Some(ref widget) = *fighter_hud_config_widget.borrow() {
                widget.set_available_fields(fields.clone());
                widget.set_source_summaries(summaries.clone());
            }
            if let Some(ref widget) = *synthwave_config_widget.borrow() {
                widget.set_available_fields(fields);
                widget.set_source_summaries(summaries);
            }
        } else {
            log::info!("=== Skipping combo widget init: old_source_id='{}' (need 'combination') ===", old_source_id);
        }
    }

    // Update combo config widgets when source dropdown changes to "combination"
    // Only updates widgets that exist (lazy init means only active one is created)
    {
        let lcars_widget_clone = lcars_config_widget.clone();
        let cyberpunk_widget_clone = cyberpunk_config_widget.clone();
        let material_widget_clone = material_config_widget.clone();
        let industrial_widget_clone = industrial_config_widget.clone();
        let retro_terminal_widget_clone = retro_terminal_config_widget.clone();
        let fighter_hud_widget_clone = fighter_hud_config_widget.clone();
        let combo_widget_clone = combo_config_widget.clone();
        let sources_clone = sources.clone();
        source_combo.connect_selected_notify(move |combo| {
            let selected_idx = combo.selected() as usize;
            if let Some(source_id) = sources_clone.get(selected_idx) {
                if source_id == "combination" {
                    // Update existing combo widgets with source summaries
                    let widget = combo_widget_clone.borrow();
                    let summaries = widget.get_source_summaries();
                    let fields = widget.get_available_fields();
                    drop(widget);
                    log::info!("=== Source changed to 'combination': updating existing combo widgets with {} source summaries ===", summaries.len());

                    if let Some(ref widget) = *lcars_widget_clone.borrow() {
                        widget.set_available_fields(fields.clone());
                        widget.set_source_summaries(summaries.clone());
                    }
                    if let Some(ref widget) = *cyberpunk_widget_clone.borrow() {
                        widget.set_available_fields(fields.clone());
                        widget.set_source_summaries(summaries.clone());
                    }
                    if let Some(ref widget) = *material_widget_clone.borrow() {
                        widget.set_available_fields(fields.clone());
                        widget.set_source_summaries(summaries.clone());
                    }
                    if let Some(ref widget) = *industrial_widget_clone.borrow() {
                        widget.set_available_fields(fields.clone());
                        widget.set_source_summaries(summaries.clone());
                    }
                    if let Some(ref widget) = *retro_terminal_widget_clone.borrow() {
                        widget.set_available_fields(fields.clone());
                        widget.set_source_summaries(summaries.clone());
                    }
                    if let Some(ref widget) = *fighter_hud_widget_clone.borrow() {
                        widget.set_available_fields(fields);
                        widget.set_source_summaries(summaries);
                    }
                }
            }
        });
    }

    // Show/hide config widgets based on displayer selection
    // For combo widgets, use placeholder boxes (they contain the lazily-created widgets)
    {
        let text_widget_clone = text_config_widget.clone();
        let text_label_clone = text_config_label.clone();
        let bar_widget_clone = bar_config_widget.clone();
        let bar_label_clone = bar_config_label.clone();
        let arc_widget_clone = arc_config_widget.clone();
        let arc_label_clone = arc_config_label.clone();
        let speedometer_widget_clone = speedometer_config_widget.clone();
        let speedometer_label_clone = speedometer_config_label.clone();
        let graph_widget_clone = graph_config_widget.clone();
        let graph_label_clone = graph_config_label.clone();
        let clock_analog_widget_clone = clock_analog_config_widget.clone();
        let clock_analog_label_clone = clock_analog_config_label.clone();
        let clock_digital_widget_clone = clock_digital_config_widget.clone();
        let clock_digital_label_clone = clock_digital_config_label.clone();
        let lcars_placeholder_clone = lcars_placeholder.clone();
        let lcars_label_clone = lcars_config_label.clone();
        let cpu_cores_widget_clone = cpu_cores_config_widget.clone();
        let cpu_cores_label_clone = cpu_cores_config_label.clone();
        let indicator_widget_clone = indicator_config_widget.clone();
        let indicator_label_clone = indicator_config_label.clone();
        let cyberpunk_placeholder_clone = cyberpunk_placeholder.clone();
        let cyberpunk_label_clone = cyberpunk_config_label.clone();
        let material_placeholder_clone = material_placeholder.clone();
        let material_label_clone = material_config_label.clone();
        let industrial_placeholder_clone = industrial_placeholder.clone();
        let industrial_label_clone = industrial_config_label.clone();
        let retro_terminal_placeholder_clone = retro_terminal_placeholder.clone();
        let retro_terminal_label_clone = retro_terminal_config_label.clone();
        let fighter_hud_placeholder_clone = fighter_hud_placeholder.clone();
        let fighter_hud_label_clone = fighter_hud_config_label.clone();
        let synthwave_placeholder_clone = synthwave_placeholder.clone();
        let synthwave_label_clone = synthwave_config_label.clone();
        let displayers_clone = displayers.clone();
        displayer_combo.connect_selected_notify(move |combo| {
            let selected_idx = combo.selected() as usize;
            if let Some(displayer_id) = displayers_clone.get(selected_idx) {
                let is_text = displayer_id == "text";
                let is_bar = displayer_id == "bar";
                let is_arc = displayer_id == "arc";
                let is_speedometer = displayer_id == "speedometer";
                let is_graph = displayer_id == "graph";
                let is_clock_analog = displayer_id == "clock_analog";
                let is_clock_digital = displayer_id == "clock_digital";
                let is_lcars = displayer_id == "lcars";
                let is_cpu_cores = displayer_id == "cpu_cores";
                let is_indicator = displayer_id == "indicator";
                let is_cyberpunk = displayer_id == "cyberpunk";
                let is_material = displayer_id == "material";
                let is_industrial = displayer_id == "industrial";
                let is_retro_terminal = displayer_id == "retro_terminal";
                let is_fighter_hud = displayer_id == "fighter_hud";
                let is_synthwave = displayer_id == "synthwave";
                text_widget_clone.widget().set_visible(is_text);
                text_label_clone.set_visible(is_text);
                bar_widget_clone.widget().set_visible(is_bar);
                bar_label_clone.set_visible(is_bar);
                arc_widget_clone.widget().set_visible(is_arc);
                arc_label_clone.set_visible(is_arc);
                speedometer_widget_clone.widget().set_visible(is_speedometer);
                speedometer_label_clone.set_visible(is_speedometer);
                graph_widget_clone.widget().set_visible(is_graph);
                graph_label_clone.set_visible(is_graph);
                clock_analog_widget_clone.widget().set_visible(is_clock_analog);
                clock_analog_label_clone.set_visible(is_clock_analog);
                clock_digital_widget_clone.widget().set_visible(is_clock_digital);
                clock_digital_label_clone.set_visible(is_clock_digital);
                lcars_placeholder_clone.set_visible(is_lcars);
                lcars_label_clone.set_visible(is_lcars);
                cpu_cores_widget_clone.widget().set_visible(is_cpu_cores);
                cpu_cores_label_clone.set_visible(is_cpu_cores);
                indicator_widget_clone.widget().set_visible(is_indicator);
                indicator_label_clone.set_visible(is_indicator);
                cyberpunk_placeholder_clone.set_visible(is_cyberpunk);
                cyberpunk_label_clone.set_visible(is_cyberpunk);
                material_placeholder_clone.set_visible(is_material);
                material_label_clone.set_visible(is_material);
                industrial_placeholder_clone.set_visible(is_industrial);
                industrial_label_clone.set_visible(is_industrial);
                retro_terminal_placeholder_clone.set_visible(is_retro_terminal);
                retro_terminal_label_clone.set_visible(is_retro_terminal);
                fighter_hud_placeholder_clone.set_visible(is_fighter_hud);
                fighter_hud_label_clone.set_visible(is_fighter_hud);
                synthwave_placeholder_clone.set_visible(is_synthwave);
                synthwave_label_clone.set_visible(is_synthwave);
            }
        });
    }

    // Lazily create and update LCARS widget when displayer changes to "lcars" and source is "combination"
    {
        let lcars_widget_clone = lcars_config_widget.clone();
        let lcars_placeholder_clone = lcars_placeholder.clone();
        let combo_widget_clone = combo_config_widget.clone();
        let displayers_clone = displayers.clone();
        let sources_clone = sources.clone();
        let source_combo_clone = source_combo.clone();
        displayer_combo.connect_selected_notify(move |combo| {
            let selected_idx = combo.selected() as usize;
            if let Some(displayer_id) = displayers_clone.get(selected_idx) {
                if displayer_id == "lcars" {
                    let source_idx = source_combo_clone.selected() as usize;
                    if let Some(source_id) = sources_clone.get(source_idx) {
                        if source_id == "combination" {
                            let combo_widget = combo_widget_clone.borrow();
                            let summaries = combo_widget.get_source_summaries();
                            let fields = combo_widget.get_available_fields();
                            drop(combo_widget);

                            // Lazily create widget if it doesn't exist
                            let mut widget_ref = lcars_widget_clone.borrow_mut();
                            if widget_ref.is_none() {
                                log::info!("=== Lazy-creating LcarsConfigWidget on displayer switch ===");
                                let widget = crate::ui::LcarsConfigWidget::new(fields.clone());
                                lcars_placeholder_clone.append(widget.widget());
                                *widget_ref = Some(widget);
                            }

                            if let Some(ref widget) = *widget_ref {
                                log::info!("=== Displayer changed to 'lcars': updating with {} source summaries ===", summaries.len());
                                widget.set_available_fields(fields);
                                widget.set_source_summaries(summaries);
                            }
                        }
                    }
                }
            }
        });
    }

    // Lazily create and update Cyberpunk widget when displayer changes to "cyberpunk" and source is "combination"
    {
        let cyberpunk_widget_clone = cyberpunk_config_widget.clone();
        let cyberpunk_placeholder_clone = cyberpunk_placeholder.clone();
        let combo_widget_clone = combo_config_widget.clone();
        let displayers_clone = displayers.clone();
        let sources_clone = sources.clone();
        let source_combo_clone = source_combo.clone();
        displayer_combo.connect_selected_notify(move |combo| {
            let selected_idx = combo.selected() as usize;
            if let Some(displayer_id) = displayers_clone.get(selected_idx) {
                if displayer_id == "cyberpunk" {
                    let source_idx = source_combo_clone.selected() as usize;
                    if let Some(source_id) = sources_clone.get(source_idx) {
                        if source_id == "combination" {
                            let combo_widget = combo_widget_clone.borrow();
                            let summaries = combo_widget.get_source_summaries();
                            let fields = combo_widget.get_available_fields();
                            drop(combo_widget);

                            // Lazily create widget if it doesn't exist
                            let mut widget_ref = cyberpunk_widget_clone.borrow_mut();
                            if widget_ref.is_none() {
                                log::info!("=== Lazy-creating CyberpunkConfigWidget on displayer switch ===");
                                let widget = crate::ui::CyberpunkConfigWidget::new(fields.clone());
                                widget.set_on_change(|| {});
                                cyberpunk_placeholder_clone.append(widget.widget());
                                *widget_ref = Some(widget);
                            }

                            if let Some(ref widget) = *widget_ref {
                                log::info!("=== Displayer changed to 'cyberpunk': updating with {} source summaries ===", summaries.len());
                                widget.set_available_fields(fields);
                                widget.set_source_summaries(summaries);
                            }
                        }
                    }
                }
            }
        });
    }

    // Lazily create and update Material widget when displayer changes to "material" and source is "combination"
    {
        let material_widget_clone = material_config_widget.clone();
        let material_placeholder_clone = material_placeholder.clone();
        let combo_widget_clone = combo_config_widget.clone();
        let displayers_clone = displayers.clone();
        let sources_clone = sources.clone();
        let source_combo_clone = source_combo.clone();
        displayer_combo.connect_selected_notify(move |combo| {
            let selected_idx = combo.selected() as usize;
            if let Some(displayer_id) = displayers_clone.get(selected_idx) {
                if displayer_id == "material" {
                    let source_idx = source_combo_clone.selected() as usize;
                    if let Some(source_id) = sources_clone.get(source_idx) {
                        if source_id == "combination" {
                            let combo_widget = combo_widget_clone.borrow();
                            let summaries = combo_widget.get_source_summaries();
                            let fields = combo_widget.get_available_fields();
                            drop(combo_widget);

                            // Lazily create widget if it doesn't exist
                            let mut widget_ref = material_widget_clone.borrow_mut();
                            if widget_ref.is_none() {
                                log::info!("=== Lazy-creating MaterialConfigWidget on displayer switch ===");
                                let widget = crate::ui::MaterialConfigWidget::new(fields.clone());
                                widget.set_on_change(|| {});
                                material_placeholder_clone.append(widget.widget());
                                *widget_ref = Some(widget);
                            }

                            if let Some(ref widget) = *widget_ref {
                                log::info!("=== Displayer changed to 'material': updating with {} source summaries ===", summaries.len());
                                widget.set_available_fields(fields);
                                widget.set_source_summaries(summaries);
                            }
                        }
                    }
                }
            }
        });
    }

    // Lazily create and update Industrial widget when displayer changes to "industrial" and source is "combination"
    {
        let industrial_widget_clone = industrial_config_widget.clone();
        let industrial_placeholder_clone = industrial_placeholder.clone();
        let combo_widget_clone = combo_config_widget.clone();
        let displayers_clone = displayers.clone();
        let sources_clone = sources.clone();
        let source_combo_clone = source_combo.clone();
        displayer_combo.connect_selected_notify(move |combo| {
            let selected_idx = combo.selected() as usize;
            if let Some(displayer_id) = displayers_clone.get(selected_idx) {
                if displayer_id == "industrial" {
                    let source_idx = source_combo_clone.selected() as usize;
                    if let Some(source_id) = sources_clone.get(source_idx) {
                        if source_id == "combination" {
                            let combo_widget = combo_widget_clone.borrow();
                            let summaries = combo_widget.get_source_summaries();
                            let fields = combo_widget.get_available_fields();
                            drop(combo_widget);

                            // Lazily create widget if it doesn't exist
                            let mut widget_ref = industrial_widget_clone.borrow_mut();
                            if widget_ref.is_none() {
                                log::info!("=== Lazy-creating IndustrialConfigWidget on displayer switch ===");
                                let widget = crate::ui::IndustrialConfigWidget::new(fields.clone());
                                widget.set_on_change(|| {});
                                industrial_placeholder_clone.append(widget.widget());
                                *widget_ref = Some(widget);
                            }

                            if let Some(ref widget) = *widget_ref {
                                log::info!("=== Displayer changed to 'industrial': updating with {} source summaries ===", summaries.len());
                                widget.set_available_fields(fields);
                                widget.set_source_summaries(summaries);
                            }
                        }
                    }
                }
            }
        });
    }

    // Lazily create and update Retro Terminal widget when displayer changes to "retro_terminal" and source is "combination"
    {
        let retro_terminal_widget_clone = retro_terminal_config_widget.clone();
        let retro_terminal_placeholder_clone = retro_terminal_placeholder.clone();
        let combo_widget_clone = combo_config_widget.clone();
        let displayers_clone = displayers.clone();
        let sources_clone = sources.clone();
        let source_combo_clone = source_combo.clone();
        displayer_combo.connect_selected_notify(move |combo| {
            let selected_idx = combo.selected() as usize;
            if let Some(displayer_id) = displayers_clone.get(selected_idx) {
                if displayer_id == "retro_terminal" {
                    let source_idx = source_combo_clone.selected() as usize;
                    if let Some(source_id) = sources_clone.get(source_idx) {
                        if source_id == "combination" {
                            let combo_widget = combo_widget_clone.borrow();
                            let summaries = combo_widget.get_source_summaries();
                            let fields = combo_widget.get_available_fields();
                            drop(combo_widget);

                            // Lazily create widget if it doesn't exist
                            let mut widget_ref = retro_terminal_widget_clone.borrow_mut();
                            if widget_ref.is_none() {
                                log::info!("=== Lazy-creating RetroTerminalConfigWidget on displayer switch ===");
                                let widget = crate::ui::RetroTerminalConfigWidget::new(fields.clone());
                                widget.set_on_change(|| {});
                                retro_terminal_placeholder_clone.append(widget.widget());
                                *widget_ref = Some(widget);
                            }

                            if let Some(ref widget) = *widget_ref {
                                log::info!("=== Displayer changed to 'retro_terminal': updating with {} source summaries ===", summaries.len());
                                widget.set_available_fields(fields);
                                widget.set_source_summaries(summaries);
                            }
                        }
                    }
                }
            }
        });
    }

    // Lazily create and update Fighter HUD widget when displayer changes to "fighter_hud" and source is "combination"
    {
        let fighter_hud_widget_clone = fighter_hud_config_widget.clone();
        let fighter_hud_placeholder_clone = fighter_hud_placeholder.clone();
        let combo_widget_clone = combo_config_widget.clone();
        let displayers_clone = displayers.clone();
        let sources_clone = sources.clone();
        let source_combo_clone = source_combo.clone();
        displayer_combo.connect_selected_notify(move |combo| {
            let selected_idx = combo.selected() as usize;
            if let Some(displayer_id) = displayers_clone.get(selected_idx) {
                if displayer_id == "fighter_hud" {
                    let source_idx = source_combo_clone.selected() as usize;
                    if let Some(source_id) = sources_clone.get(source_idx) {
                        if source_id == "combination" {
                            let combo_widget = combo_widget_clone.borrow();
                            let summaries = combo_widget.get_source_summaries();
                            let fields = combo_widget.get_available_fields();
                            drop(combo_widget);

                            // Lazily create widget if it doesn't exist
                            let mut widget_ref = fighter_hud_widget_clone.borrow_mut();
                            if widget_ref.is_none() {
                                log::info!("=== Lazy-creating FighterHudConfigWidget on displayer switch ===");
                                let widget = crate::ui::FighterHudConfigWidget::new(fields.clone());
                                widget.set_on_change(|| {});
                                fighter_hud_placeholder_clone.append(widget.widget());
                                *widget_ref = Some(widget);
                            }

                            if let Some(ref widget) = *widget_ref {
                                log::info!("=== Displayer changed to 'fighter_hud': updating with {} source summaries ===", summaries.len());
                                widget.set_available_fields(fields);
                                widget.set_source_summaries(summaries);
                            }
                        }
                    }
                }
            }
        });
    }

    // Lazily create and update Synthwave widget when displayer changes to "synthwave" and source is "combination"
    {
        let synthwave_widget_clone = synthwave_config_widget.clone();
        let synthwave_placeholder_clone = synthwave_placeholder.clone();
        let combo_widget_clone = combo_config_widget.clone();
        let displayers_clone = displayers.clone();
        let sources_clone = sources.clone();
        let source_combo_clone = source_combo.clone();
        displayer_combo.connect_selected_notify(move |combo| {
            let selected_idx = combo.selected() as usize;
            if let Some(displayer_id) = displayers_clone.get(selected_idx) {
                if displayer_id == "synthwave" {
                    let source_idx = source_combo_clone.selected() as usize;
                    if let Some(source_id) = sources_clone.get(source_idx) {
                        if source_id == "combination" {
                            let combo_widget = combo_widget_clone.borrow();
                            let summaries = combo_widget.get_source_summaries();
                            let fields = combo_widget.get_available_fields();
                            drop(combo_widget);

                            // Lazily create widget if it doesn't exist
                            let mut widget_ref = synthwave_widget_clone.borrow_mut();
                            if widget_ref.is_none() {
                                log::info!("=== Lazy-creating SynthwaveConfigWidget on displayer switch ===");
                                let widget = crate::ui::SynthwaveConfigWidget::new(fields.clone());
                                widget.set_on_change(|| {});
                                synthwave_placeholder_clone.append(widget.widget());
                                *widget_ref = Some(widget);
                            }

                            if let Some(ref widget) = *widget_ref {
                                log::info!("=== Displayer changed to 'synthwave': updating with {} source summaries ===", summaries.len());
                                widget.set_available_fields(fields);
                                widget.set_source_summaries(summaries);
                            }
                        }
                    }
                }
            }
        });
    }

    // Update text config fields when data source changes
    {
        let _text_widget_clone = text_config_widget.clone();
        let sources_clone = sources.clone();
        source_combo.connect_selected_notify(move |combo| {
            let selected_idx = combo.selected() as usize;
            if let Some(source_id) = sources_clone.get(selected_idx) {
                // Create temporary source to get its fields
                if let Ok(temp_source) = registry.create_source(source_id) {
                    let new_fields = temp_source.fields();
                    // Note: TextLineConfigWidget doesn't have a method to update fields yet
                    // For now, this will need to be handled on next open or we need to add that method
                    // TODO: Add update_fields() method to TextLineConfigWidget
                    let _ = new_fields; // Suppress unused warning for now
                }
            }
        });
    }

    notebook.append_page(&displayer_tab_box, Some(&Label::new(Some("Display Type"))));

    // === Tab: Background ===
    let background_tab_box = GtkBox::new(Orientation::Vertical, 12);
    background_tab_box.set_margin_top(12);
    background_tab_box.set_margin_bottom(12);
    background_tab_box.set_margin_start(12);
    background_tab_box.set_margin_end(12);

    let background_widget = crate::ui::BackgroundConfigWidget::new();
    background_widget.set_config(panel_guard.background.clone());
    // Set source fields for indicator background configuration
    background_widget.set_source_fields(available_fields.clone());
    background_widget.set_is_combo_source(old_source_id == "combination");
    background_tab_box.append(background_widget.widget());

    // Wrap background_widget in Rc so we can share it with the closure
    let background_widget = Rc::new(background_widget);

    notebook.append_page(&background_tab_box, Some(&Label::new(Some("Background"))));

    // === Tab: Appearance ===
    let appearance_tab_box = GtkBox::new(Orientation::Vertical, 12);
    appearance_tab_box.set_margin_top(12);
    appearance_tab_box.set_margin_bottom(12);
    appearance_tab_box.set_margin_start(12);
    appearance_tab_box.set_margin_end(12);

    // Copy/Paste Style buttons
    let copy_paste_label = Label::new(Some("Panel Style"));
    copy_paste_label.add_css_class("heading");
    appearance_tab_box.append(&copy_paste_label);

    let copy_paste_box = GtkBox::new(Orientation::Horizontal, 6);
    copy_paste_box.set_margin_start(12);

    let copy_style_btn = Button::with_label("Copy Style");
    let paste_style_btn = Button::with_label("Paste Style");

    let panel_for_copy_btn = panel.clone();
    copy_style_btn.connect_clicked(move |_| {
        let panel_guard = panel_for_copy_btn.blocking_read();
        use crate::ui::{PanelStyle, CLIPBOARD};

        // Filter out source-specific config keys
        let mut displayer_config = panel_guard.config.clone();
        displayer_config.remove("cpu_config");
        displayer_config.remove("gpu_config");
        displayer_config.remove("memory_config");
        displayer_config.remove("disk_config");
        displayer_config.remove("clock_config");
        displayer_config.remove("combo_config");
        displayer_config.remove("system_temp_config");
        displayer_config.remove("fan_speed_config");
        displayer_config.remove("test_config");
        displayer_config.remove("static_text_config");

        let style = PanelStyle {
            background: panel_guard.background.clone(),
            corner_radius: panel_guard.corner_radius,
            border: panel_guard.border.clone(),
            displayer_config,
        };

        if let Ok(mut clipboard) = CLIPBOARD.lock() {
            clipboard.copy_panel_style(style);
            log::info!("Panel style copied to clipboard via button");
        }
    });

    let panel_for_paste_btn = panel.clone();
    let background_widget_paste = background_widget.clone();

    paste_style_btn.connect_clicked(move |_| {
        use crate::ui::CLIPBOARD;

        if let Ok(clipboard) = CLIPBOARD.lock() {
            if let Some(style) = clipboard.paste_panel_style() {
                let mut panel_guard = panel_for_paste_btn.blocking_write();
                // Apply the style to panel data
                panel_guard.background = style.background.clone();
                panel_guard.corner_radius = style.corner_radius;
                panel_guard.border = style.border.clone();

                // Merge displayer config (keep source-specific configs)
                for (key, value) in style.displayer_config {
                    panel_guard.config.insert(key, value);
                }

                // Update background widget UI
                background_widget_paste.set_config(style.background);

                log::info!("Panel style pasted from clipboard via button (close and reopen dialog to see all changes)");
            } else {
                log::info!("No panel style in clipboard");
            }
        }
    });

    copy_paste_box.append(&copy_style_btn);
    copy_paste_box.append(&paste_style_btn);
    appearance_tab_box.append(&copy_paste_box);

    // Corner radius
    let corner_radius_label = Label::new(Some("Corner Radius"));
    corner_radius_label.add_css_class("heading");
    appearance_tab_box.append(&corner_radius_label);

    let corner_radius_box = GtkBox::new(Orientation::Horizontal, 6);
    corner_radius_box.set_margin_start(12);
    corner_radius_box.append(&Label::new(Some("Radius:")));
    let corner_radius_spin = SpinButton::with_range(0.0, 50.0, 1.0);
    corner_radius_spin.set_value(panel_guard.corner_radius);
    corner_radius_spin.set_hexpand(true);
    corner_radius_box.append(&corner_radius_spin);
    appearance_tab_box.append(&corner_radius_box);

    // Border section
    let border_label = Label::new(Some("Border"));
    border_label.add_css_class("heading");
    border_label.set_margin_top(12);
    appearance_tab_box.append(&border_label);

    let border_enabled_check = gtk4::CheckButton::with_label("Show Border");
    border_enabled_check.set_active(panel_guard.border.enabled);
    border_enabled_check.set_margin_start(12);
    appearance_tab_box.append(&border_enabled_check);

    let border_width_box = GtkBox::new(Orientation::Horizontal, 6);
    border_width_box.set_margin_start(12);
    border_width_box.append(&Label::new(Some("Width:")));
    let border_width_spin = SpinButton::with_range(0.5, 10.0, 0.5);
    border_width_spin.set_value(panel_guard.border.width);
    border_width_spin.set_hexpand(true);
    border_width_box.append(&border_width_spin);
    appearance_tab_box.append(&border_width_box);

    let border_color_btn = Button::with_label("Border Color");
    border_color_btn.set_margin_start(12);
    appearance_tab_box.append(&border_color_btn);

    // Store border color in a shared Rc<RefCell>
    let border_color = Rc::new(RefCell::new(panel_guard.border.color));

    // Border color button handler
    {
        let border_color_clone = border_color.clone();
        let dialog_clone = dialog.clone();
        border_color_btn.connect_clicked(move |_| {
            let current_color = *border_color_clone.borrow();
            let window_opt = dialog_clone.clone().upcast::<gtk4::Window>();
            let border_color_clone2 = border_color_clone.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) = crate::ui::ColorPickerDialog::pick_color(Some(&window_opt), current_color).await {
                    *border_color_clone2.borrow_mut() = new_color;
                }
            });
        });
    }

    notebook.append_page(&appearance_tab_box, Some(&Label::new(Some("Appearance"))));

    drop(panel_guard); // Release the panel guard before closures

    // Add notebook to main vbox
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

    // Create a shared closure for applying changes
    let panel_clone = panel.clone();
    let background_widget_clone = background_widget.clone();
    let text_config_widget_clone = text_config_widget.clone();
    let bar_config_widget_clone = bar_config_widget.clone();
    let arc_config_widget_clone = arc_config_widget.clone();
    let speedometer_config_widget_clone = speedometer_config_widget.clone();
    let graph_config_widget_clone = graph_config_widget.clone();
    let cpu_config_widget_clone = cpu_config_widget.clone();
    let gpu_config_widget_clone = gpu_config_widget.clone();
    let memory_config_widget_clone = memory_config_widget.clone();
    let system_temp_config_widget_clone = system_temp_config_widget.clone();
    let fan_speed_config_widget_clone = fan_speed_config_widget.clone();
    let disk_config_widget_clone = disk_config_widget.clone();
    let clock_config_widget_clone = clock_config_widget.clone();
    let combo_config_widget_clone = combo_config_widget.clone();
    let test_config_widget_clone = test_config_widget.clone();
    let static_text_config_widget_clone = static_text_config_widget.clone();
    let clock_analog_config_widget_clone = clock_analog_config_widget.clone();
    let clock_digital_config_widget_clone = clock_digital_config_widget.clone();
    let lcars_config_widget_clone = lcars_config_widget.clone();
    let cpu_cores_config_widget_clone = cpu_cores_config_widget.clone();
    let indicator_config_widget_clone = indicator_config_widget.clone();
    let cyberpunk_config_widget_clone = cyberpunk_config_widget.clone();
    let material_config_widget_clone = material_config_widget.clone();
    let industrial_config_widget_clone = industrial_config_widget.clone();
    let retro_terminal_config_widget_clone = retro_terminal_config_widget.clone();
    let fighter_hud_config_widget_clone = fighter_hud_config_widget.clone();
    let synthwave_config_widget_clone = synthwave_config_widget.clone();
    let dialog_for_apply = dialog.clone();
    let width_spin_for_collision = width_spin.clone();
    let height_spin_for_collision = height_spin.clone();
    let scale_spin_clone = scale_spin.clone();
    let translate_x_spin_clone = translate_x_spin.clone();
    let translate_y_spin_clone = translate_y_spin.clone();
    let z_index_spin_clone = z_index_spin.clone();
    let ignore_collision_check_clone = ignore_collision_check.clone();
    let corner_radius_spin_clone = corner_radius_spin.clone();
    let border_enabled_check_clone = border_enabled_check.clone();
    let border_width_spin_clone = border_width_spin.clone();
    let border_color_clone = border_color.clone();
    let panel_states_for_apply = panel_states.clone();
    let panel_id_for_apply = panel_id.clone();
    let selected_panels_for_apply = selected_panels.clone();
    let config_for_apply = Rc::new(RefCell::new(config));
    let occupied_cells_for_apply = occupied_cells.clone();
    let container_for_apply = _container.clone();
    let panels_for_apply = panels.clone();

    let apply_changes = Rc::new(move || {
        let new_width = width_spin.value() as u32;
        let new_height = height_spin.value() as u32;

        // Get selected source and displayer by index
        let new_source_id = sources.get(source_combo.selected() as usize)
            .cloned()
            .unwrap_or_else(|| old_source_id.clone());
        let new_displayer_id = displayers.get(displayer_combo.selected() as usize)
            .cloned()
            .unwrap_or_else(|| old_displayer_id.clone());

        // Get new background config
        let new_background = background_widget_clone.get_config();

        // Get current geometry (it may have changed from previous Apply)
        let current_geometry = *old_geometry.borrow();

        // Check if anything changed
        let size_changed = new_width != current_geometry.width || new_height != current_geometry.height;
        let source_changed = new_source_id != old_source_id;
        let displayer_changed = new_displayer_id != old_displayer_id;

        // Check if background changed (we'll always apply for now, can optimize later)
        let background_changed = true;

        if !size_changed && !source_changed && !displayer_changed && !background_changed {
            // No changes to apply
            return;
        }

        // Get panel state and clone all widget references upfront to avoid borrow conflicts
        let (background_area, frame, widget) = {
            let mut states = panel_states.borrow_mut();
            let state = match states.get_mut(&panel_id) {
                Some(s) => s,
                None => {
                    log::warn!("Panel state not found for {}", panel_id);
                    return;
                }
            };

            // Clone all widget references we'll need
            (state.background_area.clone(), state.frame.clone(), state.widget.clone())
        }; // states borrow is dropped here

        // Handle size change (collision check)
        if size_changed {
            // Check if panel has ignore_collision
            let panel_ignore_collision = panel_clone.blocking_read().ignore_collision;
            let mut occupied = occupied_cells.borrow_mut();

            // Clear old occupied cells (only if panel participates in collision)
            if !panel_ignore_collision {
                for dx in 0..current_geometry.width {
                    for dy in 0..current_geometry.height {
                        occupied.remove(&(current_geometry.x + dx, current_geometry.y + dy));
                    }
                }
            }

            // Check if new size would cause collision (skip for ignore_collision panels)
            let mut has_collision = false;
            if !panel_ignore_collision {
                for dx in 0..new_width {
                    for dy in 0..new_height {
                        let cell = (current_geometry.x + dx, current_geometry.y + dy);
                        if occupied.contains(&cell) {
                            has_collision = true;
                            break;
                        }
                    }
                    if has_collision {
                        break;
                    }
                }
            }

            if has_collision {
                // Restore old occupied cells
                for dx in 0..current_geometry.width {
                    for dy in 0..current_geometry.height {
                        occupied.insert((current_geometry.x + dx, current_geometry.y + dy));
                    }
                }
                drop(occupied);

                log::warn!("Cannot resize panel: collision detected");

                // Show error dialog and revert spinners
                let error_dialog = gtk4::AlertDialog::builder()
                    .message("Cannot Resize Panel")
                    .detail("The new size would overlap with another panel. Size has been reverted.")
                    .modal(true)
                    .buttons(vec!["OK"])
                    .build();

                // Revert spinners to current values
                width_spin_for_collision.set_value(current_geometry.width as f64);
                height_spin_for_collision.set_value(current_geometry.height as f64);

                error_dialog.show(Some(&dialog_for_apply));
                return;
            }

            // Mark new cells as occupied (only if panel participates in collision)
            if !panel_ignore_collision {
                for dx in 0..new_width {
                    for dy in 0..new_height {
                        occupied.insert((current_geometry.x + dx, current_geometry.y + dy));
                    }
                }
            }
        }

        // Update panel geometry, source, displayer, and background - single lock acquisition
        // IMPORTANT: All panel updates must be done in one lock to avoid deadlock with draw callbacks
        // Use blocking_write to ensure we get the lock (updates are fast so wait is minimal)
        {
            let mut panel_guard = panel_clone.blocking_write();
            // Update size if changed
            if size_changed {
                log::info!("[RESIZE] Panel {} geometry changing from {}x{} to {}x{}",
                          panel_id, current_geometry.width, current_geometry.height,
                          new_width, new_height);
                panel_guard.geometry.width = new_width;
                panel_guard.geometry.height = new_height;
                log::info!("[RESIZE] Panel {} geometry updated to {}x{}",
                          panel_id, panel_guard.geometry.width, panel_guard.geometry.height);
            }

            // Update background if changed
            if background_changed {
                panel_guard.background = new_background;
            }

            // Update corner radius and border (always apply)
            let new_corner_radius = corner_radius_spin_clone.value();
            panel_guard.corner_radius = new_corner_radius;
            panel_guard.border.enabled = border_enabled_check_clone.is_active();
            panel_guard.border.width = border_width_spin_clone.value();
            panel_guard.border.color = *border_color_clone.borrow();

            // Update content transform (scale and translate)
            panel_guard.scale = scale_spin_clone.value();
            panel_guard.translate_x = translate_x_spin_clone.value();
            panel_guard.translate_y = translate_y_spin_clone.value();

            // Get old values for comparison
            let old_z_index = panel_guard.z_index;
            let old_ignore_collision = panel_guard.ignore_collision;

            // Update z_index and ignore_collision
            let new_z_index = z_index_spin_clone.value() as i32;
            let new_ignore_collision = ignore_collision_check_clone.is_active();
            panel_guard.z_index = new_z_index;
            panel_guard.ignore_collision = new_ignore_collision;

            // Handle ignore_collision changes
            if old_ignore_collision != new_ignore_collision {
                let geom = panel_guard.geometry;
                let mut occupied = occupied_cells_for_apply.borrow_mut();
                if new_ignore_collision {
                    // Now ignoring collision - remove cells from occupied
                    for dx in 0..geom.width {
                        for dy in 0..geom.height {
                            occupied.remove(&(geom.x + dx, geom.y + dy));
                        }
                    }
                } else {
                    // Now participating in collision - add cells to occupied
                    for dx in 0..geom.width {
                        for dy in 0..geom.height {
                            occupied.insert((geom.x + dx, geom.y + dy));
                        }
                    }
                }
            }

            // Reorder panels if z_index changed
            let z_index_changed = old_z_index != new_z_index;

            // Update source if changed
            if source_changed {
                // Release old shared source if present
                if let Some(ref old_key) = panel_guard.source_key {
                    if let Some(manager) = crate::core::global_shared_source_manager() {
                        manager.release_source(old_key, &panel_id);
                    }
                    panel_guard.source_key = None;
                }

                match registry.create_source(&new_source_id) {
                    Ok(new_source) => {
                        panel_guard.source = new_source;

                        // Register with shared source manager for the new source
                        // Get the actual config from the config widget for the new source type
                        let source_config: Option<crate::core::SourceConfig> = match new_source_id.as_str() {
                            "cpu" => Some(crate::core::SourceConfig::Cpu(cpu_config_widget_clone.get_config())),
                            "gpu" => Some(crate::core::SourceConfig::Gpu(gpu_config_widget_clone.get_config())),
                            "memory" => Some(crate::core::SourceConfig::Memory(memory_config_widget_clone.get_config())),
                            "system_temp" => Some(crate::core::SourceConfig::SystemTemp(system_temp_config_widget_clone.get_config())),
                            "fan_speed" => Some(crate::core::SourceConfig::FanSpeed(fan_speed_config_widget_clone.get_config())),
                            "disk" => Some(crate::core::SourceConfig::Disk(disk_config_widget_clone.get_config())),
                            "clock" => Some(crate::core::SourceConfig::Clock(clock_config_widget_clone.get_config())),
                            "combination" => Some(crate::core::SourceConfig::Combo(combo_config_widget_clone.borrow().get_config())),
                            "test" => Some(crate::core::SourceConfig::Test(test_config_widget_clone.get_config())),
                            "static_text" => Some(crate::core::SourceConfig::StaticText(static_text_config_widget_clone.get_config())),
                            _ => crate::core::SourceConfig::default_for_type(&new_source_id),
                        };

                        if let Some(config) = source_config {
                            if let Some(manager) = crate::core::global_shared_source_manager() {
                                match manager.get_or_create_source(&config, &panel_id, registry) {
                                    Ok(key) => {
                                        log::debug!("Panel {} updated to shared source {}", panel_id, key);
                                        panel_guard.source_key = Some(key);
                                    }
                                    Err(e) => {
                                        log::warn!("Failed to create shared source for panel {}: {}", panel_id, e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to create source {}: {}", new_source_id, e);
                    }
                }
            }

            // Update displayer if changed
            if displayer_changed {
                match registry.create_displayer(&new_displayer_id) {
                    Ok(new_displayer) => {
                        // Create new widget from new displayer
                        let new_widget = new_displayer.create_widget();

                        // Calculate pixel dimensions
                        let pixel_width = panel_guard.geometry.width as i32 * config.cell_width
                            + (panel_guard.geometry.width as i32 - 1) * config.spacing;
                        let pixel_height = panel_guard.geometry.height as i32 * config.cell_height
                            + (panel_guard.geometry.height as i32 - 1) * config.spacing;
                        new_widget.set_size_request(pixel_width, pixel_height);

                        // Replace widget in frame
                        frame.set_child(Some(&new_widget));

                        // Update panel displayer
                        panel_guard.displayer = new_displayer;

                        // Update panel state widget reference (need to re-borrow panel_states)
                        if let Ok(mut states) = panel_states_for_apply.try_borrow_mut() {
                            if let Some(state) = states.get_mut(&panel_id_for_apply) {
                                state.widget = new_widget.clone();
                            }
                        }

                        // Re-attach gesture controllers to the new widget
                        // This is necessary because the old widget with its gesture controllers was replaced

                        // 1. Click gesture for selection
                        let gesture_click = gtk4::GestureClick::new();
                        let panel_states_click = panel_states_for_apply.clone();
                        let selected_panels_click = selected_panels_for_apply.clone();
                        let panel_id_click = panel_id_for_apply.clone();
                        let frame_click = frame.clone();

                        gesture_click.connect_pressed(move |gesture, _, _, _| {
                            let modifiers = gesture.current_event_state();
                            let ctrl_pressed = modifiers.contains(gtk4::gdk::ModifierType::CONTROL_MASK);

                            if let Ok(mut states) = panel_states_click.try_borrow_mut() {
                                let mut selected = selected_panels_click.borrow_mut();

                                if ctrl_pressed {
                                    // Toggle selection
                                    if selected.contains(&panel_id_click) {
                                        selected.remove(&panel_id_click);
                                        if let Some(state) = states.get_mut(&panel_id_click) {
                                            state.selected = false;
                                            frame_click.remove_css_class("selected");
                                        }
                                    } else {
                                        selected.insert(panel_id_click.clone());
                                        if let Some(state) = states.get_mut(&panel_id_click) {
                                            state.selected = true;
                                            frame_click.add_css_class("selected");
                                        }
                                    }
                                } else {
                                    // If clicking on an already-selected panel that's part of a multi-selection,
                                    // keep the current selection. Otherwise, clear and select only this panel
                                    if !selected.contains(&panel_id_click) || selected.len() == 1 {
                                        // Clear other selections
                                        for (id, state) in states.iter_mut() {
                                            if state.selected && id != &panel_id_click {
                                                state.selected = false;
                                                state.frame.remove_css_class("selected");
                                            }
                                        }
                                        selected.clear();

                                        // Select this panel
                                        selected.insert(panel_id_click.clone());
                                        if let Some(state) = states.get_mut(&panel_id_click) {
                                            state.selected = true;
                                            frame_click.add_css_class("selected");
                                        }
                                    }
                                }
                            }
                        });

                        new_widget.add_controller(gesture_click);

                        // 2. Right-click context menu with actions
                        use gtk4::gio;
                        let menu = gio::Menu::new();

                        // Section 1: Properties
                        let section1 = gio::Menu::new();
                        section1.append(Some("Properties..."), Some("panel.properties"));
                        menu.append_section(None, &section1);

                        // Section 2: Copy/Paste Style
                        let section2 = gio::Menu::new();
                        section2.append(Some("Copy Style"), Some("panel.copy_style"));
                        section2.append(Some("Paste Style"), Some("panel.paste_style"));
                        menu.append_section(None, &section2);

                        // Section 3: Save to File
                        let section3 = gio::Menu::new();
                        section3.append(Some("Save Panel to File..."), Some("panel.save_to_file"));
                        menu.append_section(None, &section3);

                        // Section 4: Delete
                        let section4 = gio::Menu::new();
                        section4.append(Some("Delete"), Some("panel.delete"));
                        menu.append_section(None, &section4);

                        let popover_menu = gtk4::PopoverMenu::from_model(Some(&menu));
                        popover_menu.set_parent(&new_widget);
                        popover_menu.set_has_arrow(false);

                        // Setup action group for this panel
                        let action_group = gio::SimpleActionGroup::new();

                        // Properties action
                        let panel_props = panel_clone.clone();
                        let panel_id_props = panel_id_for_apply.clone();
                        let config_props = config_for_apply.clone();
                        let panel_states_props = panel_states_for_apply.clone();
                        let occupied_cells_props = occupied_cells_for_apply.clone();
                        let container_props = container_for_apply.clone();
                        let on_change_props = on_change.clone();
                        let drop_zone_props = drop_zone.clone();
                        let selected_panels_props = selected_panels_for_apply.clone();
                        let panels_props = panels_for_apply.clone();

                        let properties_action = gio::SimpleAction::new("properties", None);
                        properties_action.connect_activate(move |_, _| {
                            log::info!("Opening properties dialog for panel: {}", panel_id_props);
                            let registry = crate::core::global_registry();
                            show_panel_properties_dialog(
                                &panel_props,
                                *config_props.borrow(),
                                panel_states_props.clone(),
                                occupied_cells_props.clone(),
                                container_props.clone(),
                                on_change_props.clone(),
                                drop_zone_props.clone(),
                                registry,
                                selected_panels_props.clone(),
                                panels_props.clone(),
                            );
                        });
                        action_group.add_action(&properties_action);

                        // Copy Style action
                        let copy_style_action = gio::SimpleAction::new("copy_style", None);
                        let panel_copy_style = panel_clone.clone();
                        copy_style_action.connect_activate(move |_, _| {
                            log::info!("Copying panel style");
                            let panel_guard = panel_copy_style.blocking_read();
                            use crate::ui::{PanelStyle, CLIPBOARD};

                            let mut displayer_config = panel_guard.config.clone();
                            displayer_config.remove("cpu_config");
                            displayer_config.remove("gpu_config");
                            displayer_config.remove("memory_config");
                            displayer_config.remove("disk_config");
                            displayer_config.remove("clock_config");
                            displayer_config.remove("combo_config");
                            displayer_config.remove("system_temp_config");
                            displayer_config.remove("fan_speed_config");
                            displayer_config.remove("test_config");

                            let style = PanelStyle {
                                background: panel_guard.background.clone(),
                                corner_radius: panel_guard.corner_radius,
                                border: panel_guard.border.clone(),
                                displayer_config,
                            };

                            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                                clipboard.copy_panel_style(style);
                                log::info!("Panel style copied to clipboard");
                            }
                        });
                        action_group.add_action(&copy_style_action);

                        // Paste Style action
                        let paste_style_action = gio::SimpleAction::new("paste_style", None);
                        let panel_paste_style = panel_clone.clone();
                        let panel_states_paste = panel_states_for_apply.clone();
                        let on_change_paste = on_change.clone();
                        let drop_zone_paste = drop_zone.clone();
                        paste_style_action.connect_activate(move |_, _| {
                            use crate::ui::CLIPBOARD;

                            if let Ok(clipboard) = CLIPBOARD.lock() {
                                if let Some(style) = clipboard.paste_panel_style() {
                                    log::info!("Pasting panel style");

                                    // Apply style and get panel ID, then drop the lock before calling callbacks
                                    let panel_id = {
                                        let mut panel_guard = panel_paste_style.blocking_write();
                                        panel_guard.background = style.background;
                                        panel_guard.corner_radius = style.corner_radius;
                                        panel_guard.border = style.border;

                                        for (key, value) in style.displayer_config {
                                            panel_guard.config.insert(key, value);
                                        }

                                        let config_clone = panel_guard.config.clone();
                                        let _ = panel_guard.displayer.apply_config(&config_clone);

                                        panel_guard.id.clone()
                                    }; // panel_guard dropped here

                                    // Trigger redraw (after releasing panel lock)
                                    if let Some(state) = panel_states_paste.borrow().get(&panel_id) {
                                        state.background_area.queue_draw();
                                        state.widget.queue_draw();
                                    }

                                    // Trigger on_change callback (after releasing panel lock to avoid deadlock)
                                    if let Some(ref callback) = *on_change_paste.borrow() {
                                        callback();
                                    }

                                    drop_zone_paste.queue_draw();
                                    log::info!("Panel style pasted successfully");
                                } else {
                                    log::info!("No panel style in clipboard");
                                }
                            }
                        });
                        action_group.add_action(&paste_style_action);

                        // Save to File action
                        let save_to_file_action = gio::SimpleAction::new("save_to_file", None);
                        let panel_save_file = panel_clone.clone();
                        let container_for_save = container_for_apply.clone();
                        save_to_file_action.connect_activate(move |_, _| {
                            log::info!("Saving panel to file");

                            let panel_data = {
                                let panel_guard = panel_save_file.blocking_read();
                                panel_guard.to_data()
                            };

                            let data = panel_data;
                            if let Some(root) = container_for_save.root() {
                                if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                                    let window_clone = window.clone();

                                    gtk4::glib::MainContext::default().spawn_local(async move {
                                        use gtk4::FileDialog;

                                        let initial_dir = directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")
                                            .map(|d| d.config_dir().to_path_buf())
                                            .unwrap_or_else(|| std::path::PathBuf::from("/"));

                                        let json_filter = gtk4::FileFilter::new();
                                        json_filter.set_name(Some("JSON files"));
                                        json_filter.add_pattern("*.json");

                                        let all_filter = gtk4::FileFilter::new();
                                        all_filter.set_name(Some("All files"));
                                        all_filter.add_pattern("*");

                                        let filters = gio::ListStore::new::<gtk4::FileFilter>();
                                        filters.append(&json_filter);
                                        filters.append(&all_filter);

                                        let suggested_name = format!("panel_{}.json", data.id.replace("-", "_"));

                                        let file_dialog = FileDialog::builder()
                                            .title("Save Panel to File")
                                            .modal(true)
                                            .initial_folder(&gio::File::for_path(&initial_dir))
                                            .initial_name(&suggested_name)
                                            .filters(&filters)
                                            .default_filter(&json_filter)
                                            .build();

                                        match file_dialog.save_future(Some(&window_clone)).await {
                                            Ok(file) => {
                                                if let Some(path) = file.path() {
                                                    log::info!("Saving panel to {:?}", path);

                                                    match serde_json::to_string_pretty(&data) {
                                                        Ok(json) => {
                                                            match std::fs::write(&path, json) {
                                                                Ok(()) => {
                                                                    log::info!("Panel saved successfully to {:?}", path);
                                                                }
                                                                Err(e) => {
                                                                    log::warn!("Failed to write panel file: {}", e);
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            log::warn!("Failed to serialize panel data: {}", e);
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                log::info!("Save panel dialog cancelled or failed: {}", e);
                                            }
                                        }
                                    });
                                }
                            }
                        });
                        action_group.add_action(&save_to_file_action);

                        // Delete action - deletes all selected panels
                        let panel_id_del = panel_id_for_apply.clone();
                        let selected_panels_del = selected_panels_for_apply.clone();
                        let panel_states_del = panel_states_for_apply.clone();
                        let occupied_cells_del = occupied_cells_for_apply.clone();
                        let container_del = container_for_apply.clone();
                        let on_change_del = on_change.clone();
                        let panels_del = panels_for_apply.clone();

                        let delete_action = gio::SimpleAction::new("delete", None);
                        delete_action.connect_activate(move |_, _| {
                            // Get all selected panels, or just the clicked panel if none selected
                            let selected = selected_panels_del.borrow();
                            let panel_ids: Vec<String> = if selected.is_empty() || !selected.contains(&panel_id_del) {
                                vec![panel_id_del.clone()]
                            } else {
                                selected.iter().cloned().collect()
                            };
                            let count = panel_ids.len();
                            drop(selected);

                            log::info!("Delete requested for {} panel(s)", count);

                            // Show confirmation dialog
                            let dialog = gtk4::AlertDialog::builder()
                                .message(format!("Delete {} Panel{}?", count, if count > 1 { "s" } else { "" }))
                                .detail(format!("This will permanently delete the selected panel{}.", if count > 1 { "s" } else { "" }))
                                .modal(true)
                                .buttons(vec!["Cancel", "Delete"])
                                .default_button(0)
                                .cancel_button(0)
                                .build();

                            let selected_panels_for_delete = selected_panels_del.clone();
                            let panel_states_for_delete = panel_states_del.clone();
                            let occupied_cells_for_delete = occupied_cells_del.clone();
                            let container_for_delete = container_del.clone();
                            let on_change_for_delete = on_change_del.clone();
                            let panels_for_delete = panels_del.clone();

                            // Get parent window for dialog
                            if let Some(root) = container_del.root() {
                                if let Some(window) = root.downcast_ref::<gtk4::Window>() {
                                    dialog.choose(Some(window), gtk4::gio::Cancellable::NONE, move |response| {
                                        if let Ok(1) = response {
                                            delete_selected_panels(
                                                &panel_ids,
                                                &selected_panels_for_delete,
                                                &panel_states_for_delete,
                                                &occupied_cells_for_delete,
                                                &container_for_delete,
                                                &panels_for_delete,
                                                &on_change_for_delete,
                                            );
                                        }
                                    });
                                }
                            }
                        });
                        action_group.add_action(&delete_action);

                        new_widget.insert_action_group("panel", Some(&action_group));

                        // Right-click gesture
                        let gesture_secondary = gtk4::GestureClick::new();
                        gesture_secondary.set_button(3); // Right mouse button

                        let popover_clone = popover_menu.clone();
                        gesture_secondary.connect_pressed(move |gesture, _, x, y| {
                            popover_clone.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(
                                x as i32,
                                y as i32,
                                1,
                                1,
                            )));
                            popover_clone.popup();
                            gesture.set_state(gtk4::EventSequenceState::Claimed);
                        });

                        new_widget.add_controller(gesture_secondary);

                        // Note: Drag gesture is attached to the frame, not the widget, so it doesn't need to be re-attached
                    }
                    Err(e) => {
                        log::warn!("Failed to create displayer {}: {}", new_displayer_id, e);
                    }
                }
            }

            // Apply text configuration if text displayer is active
            if new_displayer_id == "text" {
                let text_config = text_config_widget_clone.get_config();
                if let Ok(text_config_json) = serde_json::to_value(&text_config) {
                    panel_guard.config.insert("text_config".to_string(), text_config_json);

                    // Clone config before applying
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the displayer
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply text config: {}", e);
                    }
                }
            }

            // Apply bar configuration if bar displayer is active
            if new_displayer_id == "bar" {
                let bar_config = bar_config_widget_clone.get_config();
                if let Ok(bar_config_json) = serde_json::to_value(&bar_config) {
                    panel_guard.config.insert("bar_config".to_string(), bar_config_json);

                    // Clone config before applying
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the displayer
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply bar config: {}", e);
                    }
                }
            }

            // Apply arc configuration if arc displayer is active
            if new_displayer_id == "arc" {
                let arc_config = arc_config_widget_clone.get_config();
                if let Ok(arc_config_json) = serde_json::to_value(&arc_config) {
                    panel_guard.config.insert("arc_config".to_string(), arc_config_json);

                    // Clone config before applying
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the displayer
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply arc config: {}", e);
                    }
                }
            }

            // Apply speedometer configuration if speedometer displayer is active
            if new_displayer_id == "speedometer" {
                let speedometer_config = speedometer_config_widget_clone.get_config();
                if let Ok(speedometer_config_json) = serde_json::to_value(&speedometer_config) {
                    panel_guard.config.insert("speedometer_config".to_string(), speedometer_config_json);

                    // Clone config before applying
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the displayer
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply speedometer config: {}", e);
                    }
                }
            }

            // Apply graph configuration if graph displayer is active
            if new_displayer_id == "graph" {
                let graph_config = graph_config_widget_clone.get_config();
                if let Ok(graph_config_json) = serde_json::to_value(&graph_config) {
                    panel_guard.config.insert("graph_config".to_string(), graph_config_json);

                    // Clone config before applying
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the displayer
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply graph config: {}", e);
                    }
                }
            }

            // Apply analog clock configuration if clock_analog displayer is active
            if new_displayer_id == "clock_analog" {
                let clock_config = clock_analog_config_widget_clone.get_config();
                if let Ok(clock_config_json) = serde_json::to_value(&clock_config) {
                    panel_guard.config.insert("clock_analog_config".to_string(), clock_config_json);

                    // Clone config before applying
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the displayer
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply analog clock config: {}", e);
                    }
                }
            }

            // Apply digital clock configuration if clock_digital displayer is active
            if new_displayer_id == "clock_digital" {
                let clock_config = clock_digital_config_widget_clone.get_config();
                if let Ok(clock_config_json) = serde_json::to_value(&clock_config) {
                    panel_guard.config.insert("clock_digital_config".to_string(), clock_config_json);

                    // Clone config before applying
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the displayer
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply digital clock config: {}", e);
                    }
                }
            }

            // Apply LCARS configuration if lcars displayer is active
            if new_displayer_id == "lcars" {
                if let Some(ref widget) = *lcars_config_widget_clone.borrow() {
                    let lcars_config = widget.get_config();
                    if let Ok(lcars_config_json) = serde_json::to_value(&lcars_config) {
                        panel_guard.config.insert("lcars_config".to_string(), lcars_config_json);

                        // Clone config before applying
                        let config_clone = panel_guard.config.clone();

                        // Apply the configuration to the displayer
                        if let Err(e) = panel_guard.apply_config(config_clone) {
                            log::warn!("Failed to apply LCARS config: {}", e);
                        }
                    }
                }
            }

            // Apply CPU Cores configuration if cpu_cores displayer is active
            if new_displayer_id == "cpu_cores" {
                let core_bars_config = cpu_cores_config_widget_clone.get_config();
                if let Ok(core_bars_config_json) = serde_json::to_value(&core_bars_config) {
                    panel_guard.config.insert("core_bars_config".to_string(), core_bars_config_json);

                    // Clone config before applying
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the displayer
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply CPU Cores config: {}", e);
                    }
                }
            }

            // Apply Indicator configuration if indicator displayer is active
            if new_displayer_id == "indicator" {
                let indicator_config = indicator_config_widget_clone.get_config();
                if let Ok(indicator_config_json) = serde_json::to_value(&indicator_config) {
                    panel_guard.config.insert("indicator_config".to_string(), indicator_config_json);

                    // Clone config before applying
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the displayer
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply Indicator config: {}", e);
                    }
                }
            }

            // Apply Cyberpunk configuration if cyberpunk displayer is active
            if new_displayer_id == "cyberpunk" {
                if let Some(ref widget) = *cyberpunk_config_widget_clone.borrow() {
                    let cyberpunk_config = widget.get_config();
                    if let Ok(cyberpunk_config_json) = serde_json::to_value(&cyberpunk_config) {
                        panel_guard.config.insert("cyberpunk_config".to_string(), cyberpunk_config_json);

                        // Clone config before applying
                        let config_clone = panel_guard.config.clone();

                        // Apply the configuration to the displayer
                        if let Err(e) = panel_guard.apply_config(config_clone) {
                            log::warn!("Failed to apply Cyberpunk config: {}", e);
                        }
                    }
                }
            }

            // Apply Material configuration if material displayer is active
            if new_displayer_id == "material" {
                if let Some(ref widget) = *material_config_widget_clone.borrow() {
                    let material_config = widget.get_config();
                    if let Ok(material_config_json) = serde_json::to_value(&material_config) {
                        panel_guard.config.insert("material_config".to_string(), material_config_json);

                        // Clone config before applying
                        let config_clone = panel_guard.config.clone();

                        // Apply the configuration to the displayer
                        if let Err(e) = panel_guard.apply_config(config_clone) {
                            log::warn!("Failed to apply Material config: {}", e);
                        }
                    }
                }
            }

            // Apply Industrial configuration if industrial displayer is active
            if new_displayer_id == "industrial" {
                if let Some(ref widget) = *industrial_config_widget_clone.borrow() {
                    let industrial_config = widget.get_config();
                    if let Ok(industrial_config_json) = serde_json::to_value(&industrial_config) {
                        panel_guard.config.insert("industrial_config".to_string(), industrial_config_json);

                        // Clone config before applying
                        let config_clone = panel_guard.config.clone();

                        // Apply the configuration to the displayer
                        if let Err(e) = panel_guard.apply_config(config_clone) {
                            log::warn!("Failed to apply Industrial config: {}", e);
                        }
                    }
                }
            }

            // Apply Retro Terminal configuration if retro_terminal displayer is active
            if new_displayer_id == "retro_terminal" {
                if let Some(ref widget) = *retro_terminal_config_widget_clone.borrow() {
                    let retro_terminal_config = widget.get_config();
                    if let Ok(retro_terminal_config_json) = serde_json::to_value(&retro_terminal_config) {
                        panel_guard.config.insert("retro_terminal_config".to_string(), retro_terminal_config_json);

                        // Clone config before applying
                        let config_clone = panel_guard.config.clone();

                        // Apply the configuration to the displayer
                        if let Err(e) = panel_guard.apply_config(config_clone) {
                            log::warn!("Failed to apply Retro Terminal config: {}", e);
                        }
                    }
                }
            }

            // Apply Fighter HUD configuration if fighter_hud displayer is active
            if new_displayer_id == "fighter_hud" {
                if let Some(ref widget) = *fighter_hud_config_widget_clone.borrow() {
                    let fighter_hud_config = widget.get_config();
                    if let Ok(fighter_hud_config_json) = serde_json::to_value(&fighter_hud_config) {
                        panel_guard.config.insert("fighter_hud_config".to_string(), fighter_hud_config_json);

                        // Clone config before applying
                        let config_clone = panel_guard.config.clone();

                        // Apply the configuration to the displayer
                        if let Err(e) = panel_guard.apply_config(config_clone) {
                            log::warn!("Failed to apply Fighter HUD config: {}", e);
                        }
                    }
                }
            }

            // Apply Synthwave configuration if synthwave displayer is active
            if new_displayer_id == "synthwave" {
                if let Some(ref widget) = *synthwave_config_widget_clone.borrow() {
                    let synthwave_config = widget.get_config();
                    if let Ok(synthwave_config_json) = serde_json::to_value(&synthwave_config) {
                        panel_guard.config.insert("synthwave_config".to_string(), synthwave_config_json);

                        // Clone config before applying
                        let config_clone = panel_guard.config.clone();

                        // Apply the configuration to the displayer
                        if let Err(e) = panel_guard.apply_config(config_clone) {
                            log::warn!("Failed to apply Synthwave config: {}", e);
                        }
                    }
                }
            }

            // Apply CPU source configuration if CPU source is active
            if new_source_id == "cpu" {
                let cpu_config = cpu_config_widget_clone.get_config();
                if let Ok(cpu_config_json) = serde_json::to_value(&cpu_config) {
                    panel_guard.config.insert("cpu_config".to_string(), cpu_config_json);

                    // Clone config before applying to avoid borrow checker issues
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the source
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply CPU config to source: {}", e);
                    }

                    // Update the source with new configuration
                    if let Err(e) = panel_guard.update() {
                        log::warn!("Failed to update panel after config change: {}", e);
                    }
                }
            }

            // Apply GPU source configuration if GPU source is active
            if new_source_id == "gpu" {
                let gpu_config = gpu_config_widget_clone.get_config();
                if let Ok(gpu_config_json) = serde_json::to_value(&gpu_config) {
                    panel_guard.config.insert("gpu_config".to_string(), gpu_config_json);

                    // Clone config before applying to avoid borrow checker issues
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the source
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply GPU config to source: {}", e);
                    }

                    // Update the source with new configuration
                    if let Err(e) = panel_guard.update() {
                        log::warn!("Failed to update panel after config change: {}", e);
                    }
                }
            }

            // Apply Memory source configuration if Memory source is active
            if new_source_id == "memory" {
                let memory_config = memory_config_widget_clone.get_config();
                if let Ok(memory_config_json) = serde_json::to_value(&memory_config) {
                    panel_guard.config.insert("memory_config".to_string(), memory_config_json);

                    // Clone config before applying to avoid borrow checker issues
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the source
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply memory config to source: {}", e);
                    }

                    // Update the source with new configuration
                    if let Err(e) = panel_guard.update() {
                        log::warn!("Failed to update panel after config change: {}", e);
                    }
                }
            }

            // Apply System Temperature source configuration if system_temp source is active
            if new_source_id == "system_temp" {
                let system_temp_config = system_temp_config_widget_clone.get_config();
                if let Ok(system_temp_config_json) = serde_json::to_value(&system_temp_config) {
                    panel_guard.config.insert("system_temp_config".to_string(), system_temp_config_json);

                    // Clone config before applying to avoid borrow checker issues
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the source
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply system temp config to source: {}", e);
                    }

                    // Update the source with new configuration
                    if let Err(e) = panel_guard.update() {
                        log::warn!("Failed to update panel after config change: {}", e);
                    }
                }
            }

            // Apply Fan Speed source configuration if fan_speed source is active
            if new_source_id == "fan_speed" {
                let fan_speed_config = fan_speed_config_widget_clone.get_config();
                if let Ok(fan_speed_config_json) = serde_json::to_value(&fan_speed_config) {
                    panel_guard.config.insert("fan_speed_config".to_string(), fan_speed_config_json);

                    // Clone config before applying to avoid borrow checker issues
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the source
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply fan speed config to source: {}", e);
                    }

                    // Update the source with new configuration
                    if let Err(e) = panel_guard.update() {
                        log::warn!("Failed to update panel after config change: {}", e);
                    }
                }
            }

            // Apply Disk source configuration if disk source is active
            if new_source_id == "disk" {
                let disk_config = disk_config_widget_clone.get_config();
                if let Ok(disk_config_json) = serde_json::to_value(&disk_config) {
                    panel_guard.config.insert("disk_config".to_string(), disk_config_json);

                    // Clone config before applying to avoid borrow checker issues
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the source
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply disk config to source: {}", e);
                    }

                    // Update the source with new configuration
                    if let Err(e) = panel_guard.update() {
                        log::warn!("Failed to update panel after config change: {}", e);
                    }
                }
            }

            // Apply Clock source configuration if clock source is active
            if new_source_id == "clock" {
                let clock_config = clock_config_widget_clone.get_config();
                if let Ok(clock_config_json) = serde_json::to_value(&clock_config) {
                    panel_guard.config.insert("clock_config".to_string(), clock_config_json);

                    // Clone config before applying to avoid borrow checker issues
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the source
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply clock config to source: {}", e);
                    }

                    // Update the source with new configuration
                    if let Err(e) = panel_guard.update() {
                        log::warn!("Failed to update panel after config change: {}", e);
                    }
                }
            }

            // Apply Combination source configuration if combination source is active
            if new_source_id == "combination" {
                let combo_config = combo_config_widget_clone.borrow().get_config();
                if let Ok(combo_config_json) = serde_json::to_value(&combo_config) {
                    panel_guard.config.insert("combo_config".to_string(), combo_config_json);

                    // Clone config before applying to avoid borrow checker issues
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the source
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply combo config to source: {}", e);
                    }

                    // Update the source with new configuration
                    if let Err(e) = panel_guard.update() {
                        log::warn!("Failed to update panel after config change: {}", e);
                    }
                }
            }

            // Apply Test source configuration if test source is active
            if new_source_id == "test" {
                let test_config = test_config_widget_clone.get_config();
                if let Ok(test_config_json) = serde_json::to_value(&test_config) {
                    panel_guard.config.insert("test_config".to_string(), test_config_json);

                    // Clone config before applying to avoid borrow checker issues
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the source
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply test config to source: {}", e);
                    }

                    // Update the source with new configuration
                    if let Err(e) = panel_guard.update() {
                        log::warn!("Failed to update panel after config change: {}", e);
                    }
                }
            }

            // Apply Static Text source configuration if static_text source is active
            if new_source_id == "static_text" {
                let static_text_config = static_text_config_widget_clone.get_config();
                if let Ok(static_text_config_json) = serde_json::to_value(&static_text_config) {
                    panel_guard.config.insert("static_text_config".to_string(), static_text_config_json);

                    // Clone config before applying to avoid borrow checker issues
                    let config_clone = panel_guard.config.clone();

                    // Apply the configuration to the source
                    if let Err(e) = panel_guard.apply_config(config_clone) {
                        log::warn!("Failed to apply static_text config to source: {}", e);
                    }

                    // Update the source with new configuration
                    if let Err(e) = panel_guard.update() {
                        log::warn!("Failed to update panel after config change: {}", e);
                    }
                }
            }

            // Sync panel.data with the newly applied configs so UpdateManager uses the updated interval
            // This is critical because UpdateManager prefers panel.data.source_config over legacy config
            panel_guard.data = Some(panel_guard.to_data());

            // Drop the write lock BEFORE triggering any redraws to avoid deadlock
            drop(panel_guard);

            // Reorder panels by z-index if z_index changed
            if z_index_changed {
                // Collect panel IDs with their z_index and current positions
                let mut panel_info: Vec<(String, i32, f64, f64)> = Vec::new();
                let states = panel_states_for_apply.borrow();
                let config = config_for_apply.borrow();

                for (panel_id, state) in states.iter() {
                    let z_idx = state.panel.blocking_read().z_index;
                    let panel_guard = state.panel.blocking_read();
                    let x = panel_guard.geometry.x as f64 * (config.cell_width + config.spacing) as f64;
                    let y = panel_guard.geometry.y as f64 * (config.cell_height + config.spacing) as f64;
                    panel_info.push((panel_id.clone(), z_idx, x, y));
                }

                // Sort by z_index (ascending - lower z_index first means behind)
                panel_info.sort_by_key(|(_, z, _, _)| *z);

                drop(config);
                drop(states);

                // Re-add frames in z-order
                let states = panel_states_for_apply.borrow();
                for (panel_id, _, x, y) in panel_info {
                    if let Some(state) = states.get(&panel_id) {
                        container_for_apply.remove(&state.frame);
                        container_for_apply.put(&state.frame, x, y);
                    }
                }
            }
        }

        // Queue redraws AFTER releasing the panel write lock to avoid deadlock with draw callbacks
        background_area.queue_draw();
        widget.queue_draw();

        // Update widget and frame sizes if size changed (and displayer wasn't replaced)
        if size_changed && !displayer_changed {
            let pixel_width = new_width as i32 * config.cell_width
                + (new_width as i32 - 1) * config.spacing;
            let pixel_height = new_height as i32 * config.cell_height
                + (new_height as i32 - 1) * config.spacing;

            widget.set_size_request(pixel_width, pixel_height);
            frame.set_size_request(pixel_width, pixel_height);
            background_area.set_size_request(pixel_width, pixel_height);
        }

        // Trigger redraw of drop zone layer
        drop_zone.queue_draw();

        // Mark configuration as dirty
        if let Some(callback) = on_change.borrow().as_ref() {
            callback();
        }

        // Update old_geometry to reflect the new geometry for next Apply
        if size_changed {
            old_geometry.borrow_mut().width = new_width;
            old_geometry.borrow_mut().height = new_height;
        }
    });

    // Apply button - applies changes but keeps dialog open
    let apply_changes_clone = apply_changes.clone();
    apply_button.connect_clicked(move |_| {
        apply_changes_clone();
    });

    // Accept button - applies changes and closes dialog
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

    // Clear singleton reference when window closes
    dialog.connect_close_request(move |_| {
        PANEL_PROPERTIES_DIALOG.with(|dialog_ref| {
            *dialog_ref.borrow_mut() = None;
        });
        gtk4::glib::Propagation::Proceed
    });

    dialog.present();

    // Restore scroll position after dialog is presented
    // Use idle_add_local_once to ensure this runs after GTK finishes processing the dialog presentation
    if let (Some(sw), Some((h, v))) = (scrolled_window, saved_scroll) {
        gtk4::glib::idle_add_local_once(move || {
            sw.hadjustment().set_value(h);
            sw.vadjustment().set_value(v);
        });
    }
}
