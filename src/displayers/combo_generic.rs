//! Generic Combo Displayer
//!
//! A unified displayer implementation that works with any theme via the FrameRenderer trait.
//! This eliminates ~200 lines of duplicate code per theme displayer.

use anyhow::Result;
use cairo::Context;
use gtk4::{prelude::*, DrawingArea, Widget};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::core::{ConfigOption, ConfigSchema, Displayer, DisplayerConfig};
use crate::displayers::combo_displayer_base::{
    apply_combo_update, draw_content_items_generic, prepare_combo_update,
    setup_combo_animation_timer_ext, ComboDisplayData, ComboFrameConfig, ContentDrawParams,
    FrameRenderer, LayoutFrameConfig, ThemedFrameConfig,
};
use crate::displayers::layer_canvas::{LayerCache, LayerCanvas, LayerKey, LayerKind, LayerSink};
use crate::ui::group_title::render_group_title;

/// What drawing the frame yields besides the picture: where the content goes.
/// Kept so that a frame whose layer is reused (see `layer_canvas`) still
/// tells where the items are.
struct FrameLayoutCache {
    /// Size the cache was computed at
    width: i32,
    height: i32,
    /// Config version when cache was created
    config_version: u64,
    /// Content bounds from frame render
    content_bounds: (f64, f64, f64, f64),
    /// Group layouts from frame render
    group_layouts: Vec<(f64, f64, f64, f64)>,
}

/// Internal display data for the generic displayer (Send + Sync)
struct DisplayData<C: ComboFrameConfig> {
    config: C,
    combo: ComboDisplayData,
    /// Monotonically increasing counter to detect config changes
    config_version: u64,
}

impl<C: ComboFrameConfig> Default for DisplayData<C> {
    fn default() -> Self {
        Self {
            config: C::default(),
            combo: ComboDisplayData::default(),
            config_version: 0,
        }
    }
}

/// Generic combo displayer that stores the renderer in an Arc for sharing.
///
/// This provides a complete Displayer implementation by delegating theme-specific
/// rendering to a FrameRenderer. All common functionality (animation, data
/// updates, content rendering) is handled here. The Arc lets the renderer be
/// used in the draw closure without ownership issues.
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
                config_version: 0,
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
            data.config_version = data.config_version.wrapping_add(1); // Invalidate frame cache
        }
    }

    /// Render frame, dividers and content items to any Cairo context, for
    /// example an offscreen `ImageSurface` (tests, previews, mockups). The
    /// GTK widget path is `create_widget`; this shares its drawing code.
    pub fn render_to_context(&self, cr: &Context, width: i32, height: i32) {
        if let Ok(data) = self.data.lock() {
            let cache = Rc::new(RefCell::new(None));
            Self::render_frame_and_content(cr, width, height, &data, &self.renderer, &cache);
        }
    }

    /// Render the complete panel (frame and content) to one Cairo context,
    /// under the panel's transform. Every layer is drawn; the widget goes
    /// through `render_layers` with a cache instead.
    fn render_frame_and_content(
        cr: &Context,
        width: i32,
        height: i32,
        data: &DisplayData<R::Config>,
        renderer: &Arc<R>,
        layout_cache: &Rc<RefCell<Option<FrameLayoutCache>>>,
    ) {
        data.combo.transform.apply(cr, width as f64, height as f64);
        let mut sink = LayerSink::direct(cr);
        Self::render_layers(&mut sink, width, height, data, renderer, layout_cache);
        data.combo.transform.restore(cr);
    }

    /// The panel as layers (see `layer_canvas`): the frame with the group
    /// dividers, each enabled group title, each content item, the overlay
    /// effects. The frame layer also yields the content bounds and the group
    /// layouts, kept in `layout_cache` under the same key as the layer.
    fn render_layers(
        sink: &mut LayerSink,
        width: i32,
        height: i32,
        data: &DisplayData<R::Config>,
        renderer: &Arc<R>,
        layout_cache: &Rc<RefCell<Option<FrameLayoutCache>>>,
    ) {
        let w = width as f64;
        let h = height as f64;
        let version = data.config_version;

        // The frame and the dividers: one layer that only the config and the
        // size decide. Drawing the frame is what tells where the content
        // goes, so the layer leaves that in the layout cache.
        let mut chrome_key = LayerKey::new();
        chrome_key.u64(version).f64(w).f64(h);
        sink.layer(
            (LayerKind::Chrome, 0, 0),
            chrome_key.finish(),
            None,
            &|cr| {
                let content_bounds = match renderer.render_frame(cr, &data.config, w, h) {
                    Ok(bounds) => bounds,
                    Err(e) => {
                        log::debug!("{} frame render error: {}", renderer.theme_name(), e);
                        *layout_cache.borrow_mut() = None;
                        return 0.0;
                    }
                };
                let (cx, cy, cw, ch) = content_bounds;
                let group_layouts = renderer.calculate_group_layouts(&data.config, cx, cy, cw, ch);
                renderer.draw_group_dividers(cr, &data.config, &group_layouts);
                *layout_cache.borrow_mut() = Some(FrameLayoutCache {
                    width,
                    height,
                    config_version: version,
                    content_bounds,
                    group_layouts,
                });
                0.0
            },
        );

        let layout = {
            let cache_ref = layout_cache.borrow();
            cache_ref
                .as_ref()
                .filter(|c| c.width == width && c.height == height && c.config_version == version)
                .map(|c| (c.content_bounds, c.group_layouts.clone()))
        };
        // The frame could not be drawn: nothing to put content in
        let Some((content_bounds, group_layouts)) = layout else {
            return;
        };

        // Clip to content area for dynamic content
        sink.set_clip(Some(content_bounds));

        // Build draw params for dynamic content
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

        // Draw dynamic content items for each group
        let group_item_counts = data.config.group_item_counts();
        for (group_idx, &(gx, gy, gw, gh)) in group_layouts.iter().enumerate() {
            let item_count = group_item_counts.get(group_idx).copied().unwrap_or(1);
            let base_prefix = format!("group{}_", group_idx + 1);

            // An enabled title takes a band at the top of the group's
            // rectangle; the items get the rest.
            let (gy, gh) = match data.config.group_titles().get(group_idx) {
                Some(title) if title.enabled => {
                    let mut key = LayerKey::new();
                    key.u64(version).rect((gx, gy, gw, gh));
                    let used = sink.layer(
                        (LayerKind::Title, group_idx as u32, 0),
                        key.finish(),
                        None,
                        &|cr| {
                            render_group_title(cr, gx, gy, gw, gh, title, data.config.theme())
                                .unwrap_or(0.0)
                        },
                    );
                    (gy + used, gh - used)
                }
                _ => (gy, gh),
            };

            let config_ref = &data.config;
            draw_content_items_generic(
                sink,
                version,
                gx,
                gy,
                gw,
                gh,
                &base_prefix,
                item_count as u32,
                group_idx,
                &draw_params,
                |cr, x, y, w, h| {
                    renderer.draw_item_frame(cr, config_ref, x, y, w, h);
                },
            );
        }

        sink.set_clip(None);

        // Draw overlay effects (e.g., CRT scanlines, vignette) on top of all content
        let mut overlay_key = LayerKey::new();
        overlay_key.u64(version).f64(w).f64(h);
        sink.layer(
            (LayerKind::Overlay, 0, 0),
            overlay_key.finish(),
            None,
            &|cr| {
                if let Err(e) = renderer.render_overlay(cr, &data.config, w, h) {
                    log::debug!("{} overlay render error: {}", renderer.theme_name(), e);
                }
                0.0
            },
        );
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
        let canvas = LayerCanvas::new();
        canvas.set_size_request(400, 300);

        let data_clone = self.data.clone();
        let renderer_clone = self.renderer.clone();

        // Layout and layer caches live in the draw closure (not Send/Sync required)
        let layout_cache: Rc<RefCell<Option<FrameLayoutCache>>> = Rc::new(RefCell::new(None));
        let layers = Rc::new(RefCell::new(LayerCache::default()));

        // The panel is drawn as layers, so that a frame on which one item
        // moved records, rasterises and uploads that item and nothing else
        // (see `layer_canvas`). A poisoned lock draws nothing (transparent).
        canvas.set_snapshot_func(move |snapshot, width, height| {
            if width < 10 || height < 10 {
                return;
            }

            let Ok(data) = data_clone.lock() else {
                return;
            };
            let mut layers = layers.borrow_mut();
            let mut sink = LayerSink::cached(
                snapshot,
                &mut layers,
                width as f64,
                height as f64,
                &data.combo.transform,
            );
            Self::render_layers(
                &mut sink,
                width,
                height,
                &data,
                &renderer_clone,
                &layout_cache,
            );
            sink.finish();
        });
        let drawing_area: DrawingArea = canvas.upcast();

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
        // Phase 1: Brief lock to extract config
        let config_snapshot = {
            let Ok(display_data) = self.data.lock() else {
                return;
            };
            (
                display_data.config.animation_enabled(),
                display_data.config.group_item_counts().to_vec(),
                display_data.config.content_items().clone(),
                display_data.combo.graph_start_time.elapsed().as_secs_f64(),
            )
        };
        let (animation_enabled, group_item_counts, content_items, timestamp) = config_snapshot;

        // Phase 2: Expensive pre-computation (no lock held)
        let prep = prepare_combo_update(
            data,
            group_item_counts,
            &content_items,
            animation_enabled,
            timestamp,
        );

        // Phase 3: Brief lock to apply results
        if let Ok(mut display_data) = self.data.lock() {
            apply_combo_update(&mut display_data.combo, prep, data);
        }
    }

    fn draw(&self, cr: &Context, width: f64, height: f64) -> Result<()> {
        if width < 10.0 || height < 10.0 {
            return Ok(());
        }

        if let Ok(data) = self.data.try_lock() {
            data.combo.transform.apply(cr, width, height);
            self.renderer
                .render_frame(cr, &data.config, width, height)?;
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
                    display_data.config_version = display_data.config_version.wrapping_add(1);
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
            display_data.config_version = display_data.config_version.wrapping_add(1);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::PanelData;
    use crate::displayers::combo_utils;
    use crate::displayers::layer_canvas::test_support::{same_picture, slot_values};
    use crate::ui::lcars_display::ContentDisplayType;

    /// A saved panel, taken through a sequence of updates, must look the same
    /// drawn as cached layers as it does drawn in one go: a layer key that
    /// misses an input would leave an old picture of an item behind. Returns
    /// how many layers each step recorded again.
    fn layered_matches_direct<R: FrameRenderer>(
        renderer: R,
        panel: &str,
        frame_of: fn(&DisplayerConfig) -> Option<R::Config>,
    ) -> Vec<u64> {
        let (width, height) = (480, 960);
        let mut panel: PanelData = serde_json::from_str(panel).expect("panel file");
        // the frame's group counts are filled in from the source, as on load
        panel.sync_group_structure();
        let frame = frame_of(&panel.displayer_config).expect("a panel of this displayer");
        let mut displayer = GenericComboDisplayerShared::new(renderer);
        displayer.set_config(frame.clone());
        let name = displayer.renderer.theme_name();
        let counts: Vec<usize> = {
            let data = displayer.data.lock().unwrap();
            data.config.group_item_counts().to_vec()
        };
        let prefixes = combo_utils::generate_prefixes(&counts);
        assert!(!prefixes.is_empty(), "{name}: the panel has no items");

        let mut layers = LayerCache::default();
        let layout_cache = Rc::new(RefCell::new(None));
        let direct_cache = Rc::new(RefCell::new(None));
        let mut redrawn = Vec::new();
        // step 0 draws everything; 1 and 2 change a third of the slots, the
        // glide half way on 2; 3 changes nothing; 4 scales the panel (every layer anew); 5
        // applies the config again
        for step in 0..6u64 {
            let mut values = slot_values(&prefixes, &|i| match step {
                0 => 0,
                1 => u64::from(i % 3 == 0),
                2 | 3 => u64::from(i % 3 == 0) + 2 * u64::from(i % 3 == 1),
                _ => 5,
            });
            if step >= 4 {
                values.insert("_panel_scale".to_string(), Value::from(0.9));
            }
            if step == 5 {
                displayer.set_config(frame.clone());
            }
            displayer.update_data(&values);
            if step == 2 {
                let mut data = displayer.data.lock().unwrap();
                let combo = &mut data.combo;
                let bars = combo.bar_values.values_mut();
                for value in bars.chain(combo.core_bar_values.values_mut().flatten()) {
                    value.current += (value.target - value.current) * 0.5;
                }
            }
            let data = displayer.data.lock().unwrap();
            redrawn.push(same_picture(
                &format!("{name}, step {step}"),
                step >= 4,
                width,
                height,
                &mut layers,
                &|cr| {
                    GenericComboDisplayerShared::<R>::render_frame_and_content(
                        cr,
                        width,
                        height,
                        &data,
                        &displayer.renderer,
                        &direct_cache,
                    )
                },
                &|snapshot, layers| {
                    let mut sink = LayerSink::cached(
                        snapshot,
                        layers,
                        width as f64,
                        height as f64,
                        &data.combo.transform,
                    );
                    GenericComboDisplayerShared::<R>::render_layers(
                        &mut sink,
                        width,
                        height,
                        &data,
                        &displayer.renderer,
                        &layout_cache,
                    );
                    sink.finish();
                },
            ));
        }
        let all = redrawn[0];
        assert!(all > 2, "{name}: {redrawn:?}");
        assert!(redrawn[1] > 0 && redrawn[1] < all, "{name}: {redrawn:?}");
        // every update adds a point to a graph; nothing else has changed
        let graphs = {
            let data = displayer.data.lock().unwrap();
            let items = data.config.content_items();
            prefixes
                .iter()
                .filter_map(|prefix| items.get(prefix))
                .filter(|item| item.display_as == ContentDisplayType::Graph)
                .count() as u64
        };
        assert!(
            redrawn[3] <= graphs,
            "{name}: unchanged values redraw the graphs alone ({graphs}): {redrawn:?}"
        );
        assert_eq!(redrawn[5], all, "{name}: a new config redraws everything");
        redrawn
    }

    /// Every themed frame, each on its saved panel (Retro Terminal draws
    /// an overlay, its scanlines, over the items; Cyberpunk and Industrial
    /// draw a frame around each item). One test, since GTK belongs to the
    /// thread that initialised it:
    /// `cargo test --lib themed_layers_match -- --ignored`
    /// on a display (`GDK_BACKEND=broadway BROADWAY_DISPLAY=:7` with
    /// `gtk4-broadwayd :7` running does without the desktop).
    #[test]
    #[ignore = "needs a display"]
    fn themed_layers_match_the_direct_drawing() {
        gtk4::init().expect("a display");
        macro_rules! frame {
            ($renderer:expr, $file:literal, $variant:ident) => {
                layered_matches_direct(
                    $renderer,
                    include_str!(concat!("../../examples/", $file)),
                    |config| match config {
                        DisplayerConfig::$variant(c) => Some(c.to_frame()),
                        _ => None,
                    },
                );
            };
        }
        use crate::ui;
        frame!(
            ui::material_display::MaterialRenderer,
            "Material.json",
            Material
        );
        frame!(
            ui::retro_terminal_display::RetroTerminalRenderer,
            "RetroTerminal.json",
            RetroTerminal
        );
        frame!(
            ui::art_deco_display::ArtDecoRenderer,
            "Art Deco.json",
            ArtDeco
        );
        frame!(
            ui::cyberpunk_display::CyberpunkRenderer,
            "Cyberpunk.json",
            Cyberpunk
        );
        frame!(
            ui::industrial_display::IndustrialRenderer,
            "Industrial.json",
            Industrial
        );
        frame!(
            ui::fighter_hud_display::FighterHudRenderer,
            "HUD.json",
            FighterHud
        );
        frame!(
            ui::steampunk_display::SteampunkRenderer,
            "SteamPunk.json",
            Steampunk
        );
        frame!(
            ui::synthwave_display::SynthwaveRenderer,
            "Synthwave.json",
            Synthwave
        );
        frame!(
            ui::art_nouveau_display::ArtNouveauRenderer,
            "Art Nouveau.json",
            ArtNouveau
        );
    }
}
