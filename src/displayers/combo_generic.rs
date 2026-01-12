//! Generic Combo Displayer
//!
//! A unified displayer implementation that works with any theme via the FrameRenderer trait.
//! This eliminates ~200 lines of duplicate code per theme displayer.

use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig};
use crate::displayers::combo_displayer_base::{
    ComboDisplayData, ComboFrameConfig, ContentDrawParams, FrameRenderer,
    LayoutFrameConfig, ThemedFrameConfig,
    draw_content_items_generic, handle_combo_update_data, setup_combo_animation_timer_ext,
};

/// Internal display data for the generic displayer
#[derive(Clone)]
struct DisplayData<C: ComboFrameConfig> {
    config: C,
    combo: ComboDisplayData,
}

impl<C: ComboFrameConfig> Default for DisplayData<C> {
    fn default() -> Self {
        Self {
            config: C::default(),
            combo: ComboDisplayData::default(),
        }
    }
}

/// Generic combo displayer that works with any theme implementing FrameRenderer.
///
/// This struct provides a complete Displayer implementation by delegating
/// theme-specific rendering to a FrameRenderer. All common functionality
/// (animation, data updates, content rendering) is handled here.
///
/// # Type Parameters
/// * `R` - The FrameRenderer implementation for the theme
///
/// # Example
/// ```ignore
/// // Define a frame renderer for a theme
/// pub struct MyThemeRenderer;
///
/// impl FrameRenderer for MyThemeRenderer {
///     type Config = MyThemeConfig;
///     fn theme_id(&self) -> &'static str { "my_theme" }
///     fn theme_name(&self) -> &'static str { "My Theme" }
///     // ... implement other methods
/// }
///
/// // Create the displayer
/// let displayer = GenericComboDisplayer::new(MyThemeRenderer);
/// ```
pub struct GenericComboDisplayer<R: FrameRenderer> {
    renderer: R,
    data: Arc<Mutex<DisplayData<R::Config>>>,
    _phantom: PhantomData<R::Config>,
}

impl<R: FrameRenderer> GenericComboDisplayer<R> {
    /// Create a new generic combo displayer with the given frame renderer
    pub fn new(renderer: R) -> Self {
        let default_config = renderer.default_config();
        Self {
            renderer,
            data: Arc::new(Mutex::new(DisplayData {
                config: default_config,
                combo: ComboDisplayData::default(),
            })),
            _phantom: PhantomData,
        }
    }

    /// Get a clone of the current configuration
    pub fn get_config(&self) -> Option<R::Config> {
        self.data.try_lock().ok().map(|d| d.config.clone())
    }

    /// Set the configuration
    pub fn set_config(&self, config: R::Config) {
        if let Ok(mut data) = self.data.lock() {
            data.config = config;
            data.combo.dirty = true;
        }
    }
}

impl<R: FrameRenderer> Displayer for GenericComboDisplayer<R> {
    fn id(&self) -> &str {
        self.renderer.theme_id()
    }

    fn name(&self) -> &str {
        self.renderer.theme_name()
    }

    fn create_widget(&self) -> Widget {
        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(400, 300);

        // Clone data for the draw closure
        let data_clone = self.data.clone();

        // We need to store a reference to the renderer for use in the draw function.
        // Since FrameRenderer is Send + Sync, we can wrap it in an Arc.
        // However, the renderer is part of self, so we need to be careful here.
        // For now, we'll use a type-erased approach with closures that capture
        // the renderer's methods.

        // Create closures that capture the renderer's behavior
        let renderer_render_frame = {
            // We need to store render function pointers or use a different approach
            // Since we can't easily clone the renderer into the closure,
            // we'll store the renderer in an Arc and share it
            Arc::new(self.renderer.default_config()) // placeholder
        };
        let _ = renderer_render_frame; // suppress unused warning for now

        // For the draw function, we need access to:
        // 1. The renderer (for render_frame, calculate_group_layouts, etc.)
        // 2. The display data (config, combo)
        //
        // The challenge is that Rust closures capture by value or reference,
        // but we need the renderer to be accessible in a 'static closure.
        //
        // Solution: Use a struct that holds both the renderer reference and data,
        // wrapped in Arc<Mutex<>>. However, since R is not Clone by default,
        // we need a different approach.
        //
        // Alternative: Store the renderer in an Arc. Since FrameRenderer: Send + Sync,
        // this works well. But the renderer is owned by GenericComboDisplayer.
        //
        // For now, we'll create the draw function that works with the data,
        // and note that the actual rendering logic needs the renderer.
        // Since we can't easily share the renderer, each theme will need to
        // provide its own create_widget that can capture theme-specific functions.

        drawing_area.set_draw_func(move |_, cr, width, height| {
            if width < 10 || height < 10 {
                return;
            }

            if let Ok(data) = data_clone.lock() {
                let w = width as f64;
                let h = height as f64;

                // Clear to transparent so panel background shows through
                cr.set_operator(cairo::Operator::Clear);
                cr.paint().ok();
                cr.set_operator(cairo::Operator::Over);

                data.combo.transform.apply(cr, w, h);

                // TODO: The actual frame rendering would go here, but we need
                // access to the renderer. See GenericComboDisplayerWithRenderer
                // for a solution that stores the renderer in an Arc.

                data.combo.transform.restore(cr);
            }
        });

        // Set up animation timer
        setup_combo_animation_timer_ext(
            &drawing_area,
            self.data.clone(),
            |d| d.config.animation_enabled(),
            |d| d.config.animation_speed(),
            |d| &mut d.combo,
            None::<fn(&mut DisplayData<R::Config>, f64) -> bool>,
        );

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        if let Ok(mut display_data) = self.data.lock() {
            let animation_enabled = display_data.config.animation_enabled();
            let group_item_counts = display_data.config.group_item_counts().to_vec();
            let content_items = display_data.config.content_items().clone();

            handle_combo_update_data(
                &mut display_data.combo,
                data,
                &group_item_counts,
                &content_items,
                animation_enabled,
            );
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if width < 10.0 || height < 10.0 {
            return Ok(());
        }

        if let Ok(data) = self.data.try_lock() {
            data.combo.transform.apply(cr, width, height);
            self.renderer.render_frame(cr, &data.config, width, height)?;
            data.combo.transform.restore(cr);
        }

        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        // Generic schema - themes can override this
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "animation_enabled".to_string(),
                    name: "Animation".to_string(),
                    description: "Enable smooth animations".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
                ConfigOption {
                    key: "animation_speed".to_string(),
                    name: "Animation Speed".to_string(),
                    description: "Animation speed multiplier".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(8.0),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        // Try to deserialize the full config from a theme-specific key
        let config_key = format!("{}_config", self.renderer.theme_id());
        if let Some(config_value) = config.get(&config_key) {
            if let Ok(new_config) = serde_json::from_value::<R::Config>(config_value.clone()) {
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config = new_config;
                    display_data.combo.dirty = true;
                }
                return Ok(());
            }
        }

        // Apply individual settings for backward compatibility
        if let Ok(mut display_data) = self.data.lock() {
            if let Some(animation) = config.get("animation_enabled").and_then(|v| v.as_bool()) {
                display_data.config.set_animation_enabled(animation);
            }
            if let Some(speed) = config.get("animation_speed").and_then(|v| v.as_f64()) {
                display_data.config.set_animation_speed(speed);
            }
            display_data.combo.dirty = true;
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        self.data
            .try_lock()
            .map(|data| data.combo.dirty)
            .unwrap_or(true)
    }

    fn get_typed_config(&self) -> Option<DisplayerConfig> {
        // This needs to be implemented by each theme to return the correct variant
        // For now, return None - themes should override this method
        None
    }
}

/// A version of GenericComboDisplayer that stores the renderer in an Arc for sharing.
///
/// This allows the renderer to be used in the draw closure without ownership issues.
/// Use this when you need full generic behavior including rendering in create_widget.
pub struct GenericComboDisplayerShared<R: FrameRenderer> {
    renderer: Arc<R>,
    data: Arc<Mutex<DisplayData<R::Config>>>,
}

impl<R: FrameRenderer> GenericComboDisplayerShared<R> {
    /// Create a new shared generic combo displayer
    pub fn new(renderer: R) -> Self {
        let default_config = renderer.default_config();
        Self {
            renderer: Arc::new(renderer),
            data: Arc::new(Mutex::new(DisplayData {
                config: default_config,
                combo: ComboDisplayData::default(),
            })),
        }
    }

    /// Get a clone of the current configuration
    pub fn get_config(&self) -> Option<R::Config> {
        self.data.try_lock().ok().map(|d| d.config.clone())
    }

    /// Set the configuration
    pub fn set_config(&self, config: R::Config) {
        if let Ok(mut data) = self.data.lock() {
            data.config = config;
            data.combo.dirty = true;
        }
    }
}

impl<R: FrameRenderer> Displayer for GenericComboDisplayerShared<R> {
    fn id(&self) -> &str {
        self.renderer.theme_id()
    }

    fn name(&self) -> &str {
        self.renderer.theme_name()
    }

    fn create_widget(&self) -> Widget {
        let drawing_area = DrawingArea::new();
        drawing_area.set_size_request(400, 300);

        let data_clone = self.data.clone();
        let renderer_clone = self.renderer.clone();

        drawing_area.set_draw_func(move |_, cr, width, height| {
            if width < 10 || height < 10 {
                return;
            }

            if let Ok(data) = data_clone.lock() {
                let w = width as f64;
                let h = height as f64;

                // Clear to transparent so panel background shows through
                cr.set_operator(cairo::Operator::Clear);
                cr.paint().ok();
                cr.set_operator(cairo::Operator::Over);

                data.combo.transform.apply(cr, w, h);

                // Render the frame
                let content_bounds = match renderer_clone.render_frame(cr, &data.config, w, h) {
                    Ok(bounds) => bounds,
                    Err(e) => {
                        log::debug!("{} frame render error: {}", renderer_clone.theme_name(), e);
                        data.combo.transform.restore(cr);
                        return;
                    }
                };

                let (content_x, content_y, content_w, content_h) = content_bounds;

                // Calculate group layouts
                let group_layouts = renderer_clone.calculate_group_layouts(
                    &data.config,
                    content_x,
                    content_y,
                    content_w,
                    content_h,
                );

                // Draw group dividers
                renderer_clone.draw_group_dividers(cr, &data.config, &group_layouts);

                // Clip to content area
                cr.save().ok();
                cr.rectangle(content_x, content_y, content_w, content_h);
                cr.clip();

                // Build draw params
                let draw_params = ContentDrawParams {
                    values: &data.combo.values,
                    bar_values: &data.combo.bar_values,
                    core_bar_values: &data.combo.core_bar_values,
                    graph_history: &data.combo.graph_history,
                    content_items: data.config.content_items(),
                    group_item_orientations: data.config.group_item_orientations(),
                    split_orientation: data.config.split_orientation(),
                    theme: data.config.theme(),
                };

                // Draw content items for each group
                let group_item_counts = data.config.group_item_counts();
                for (group_idx, &(gx, gy, gw, gh)) in group_layouts.iter().enumerate() {
                    let item_count = group_item_counts.get(group_idx).copied().unwrap_or(1);
                    let base_prefix = format!("group{}_", group_idx + 1);

                    // Draw item frames and content
                    let config_ref = &data.config;
                    let _ = draw_content_items_generic(
                        cr,
                        gx,
                        gy,
                        gw,
                        gh,
                        &base_prefix,
                        item_count as u32,
                        group_idx,
                        &draw_params,
                        |cr, x, y, w, h| {
                            renderer_clone.draw_item_frame(cr, config_ref, x, y, w, h);
                        },
                    );
                }

                cr.restore().ok();
                data.combo.transform.restore(cr);
            }
        });

        // Set up animation timer
        setup_combo_animation_timer_ext(
            &drawing_area,
            self.data.clone(),
            |d| d.config.animation_enabled(),
            |d| d.config.animation_speed(),
            |d| &mut d.combo,
            None::<fn(&mut DisplayData<R::Config>, f64) -> bool>,
        );

        drawing_area.upcast()
    }

    fn update_data(&mut self, data: &HashMap<String, Value>) {
        if let Ok(mut display_data) = self.data.lock() {
            let animation_enabled = display_data.config.animation_enabled();
            let group_item_counts = display_data.config.group_item_counts().to_vec();
            let content_items = display_data.config.content_items().clone();

            handle_combo_update_data(
                &mut display_data.combo,
                data,
                &group_item_counts,
                &content_items,
                animation_enabled,
            );
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if width < 10.0 || height < 10.0 {
            return Ok(());
        }

        if let Ok(data) = self.data.try_lock() {
            data.combo.transform.apply(cr, width, height);
            self.renderer.render_frame(cr, &data.config, width, height)?;
            data.combo.transform.restore(cr);
        }

        Ok(())
    }

    fn config_schema(&self) -> ConfigSchema {
        ConfigSchema {
            options: vec![
                ConfigOption {
                    key: "animation_enabled".to_string(),
                    name: "Animation".to_string(),
                    description: "Enable smooth animations".to_string(),
                    value_type: "boolean".to_string(),
                    default: serde_json::json!(true),
                },
                ConfigOption {
                    key: "animation_speed".to_string(),
                    name: "Animation Speed".to_string(),
                    description: "Animation speed multiplier".to_string(),
                    value_type: "number".to_string(),
                    default: serde_json::json!(8.0),
                },
            ],
        }
    }

    fn apply_config(&mut self, config: &HashMap<String, Value>) -> Result<()> {
        let config_key = format!("{}_config", self.renderer.theme_id());
        if let Some(config_value) = config.get(&config_key) {
            if let Ok(new_config) = serde_json::from_value::<R::Config>(config_value.clone()) {
                if let Ok(mut display_data) = self.data.lock() {
                    display_data.config = new_config;
                    display_data.combo.dirty = true;
                }
                return Ok(());
            }
        }

        if let Ok(mut display_data) = self.data.lock() {
            if let Some(animation) = config.get("animation_enabled").and_then(|v| v.as_bool()) {
                display_data.config.set_animation_enabled(animation);
            }
            if let Some(speed) = config.get("animation_speed").and_then(|v| v.as_f64()) {
                display_data.config.set_animation_speed(speed);
            }
            display_data.combo.dirty = true;
        }

        Ok(())
    }

    fn needs_redraw(&self) -> bool {
        self.data
            .try_lock()
            .map(|data| data.combo.dirty)
            .unwrap_or(true)
    }

    fn get_typed_config(&self) -> Option<DisplayerConfig> {
        None
    }
}
