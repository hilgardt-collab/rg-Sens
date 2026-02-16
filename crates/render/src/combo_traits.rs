//! Combo panel rendering traits

use anyhow::Result;
use cairo::Context;
use rg_sens_types::combo::ComboFrameConfig;

/// Trait for theme-specific frame rendering.
///
/// Implementations encapsulate all visual aspects of a combo theme:
/// - Frame/border rendering
/// - Group layout calculation
/// - Divider rendering
/// - Optional item frame rendering
///
/// By implementing this trait, a theme can be used with GenericComboDisplayer.
pub trait FrameRenderer: Send + Sync + 'static {
    /// The configuration type for this theme
    type Config: ComboFrameConfig;

    /// Theme identifier used for registration and serialization (e.g., "cyberpunk", "material")
    fn theme_id(&self) -> &'static str;

    /// Human-readable theme name for display in UI (e.g., "Cyberpunk HUD", "Material Design")
    fn theme_name(&self) -> &'static str;

    /// Create a default configuration for this theme
    fn default_config(&self) -> Self::Config;

    /// Render the outer frame and background.
    ///
    /// Returns content bounds (x, y, width, height) where content items should be drawn.
    fn render_frame(
        &self,
        cr: &Context,
        config: &Self::Config,
        width: f64,
        height: f64,
    ) -> Result<(f64, f64, f64, f64)>;

    /// Calculate the layout rectangles for each group within the content area.
    ///
    /// Returns a Vec of (x, y, width, height) tuples, one per group.
    fn calculate_group_layouts(
        &self,
        config: &Self::Config,
        content_x: f64,
        content_y: f64,
        content_w: f64,
        content_h: f64,
    ) -> Vec<(f64, f64, f64, f64)>;

    /// Draw dividers between groups.
    ///
    /// Called after group layouts are calculated but before content is drawn.
    fn draw_group_dividers(
        &self,
        cr: &Context,
        config: &Self::Config,
        group_layouts: &[(f64, f64, f64, f64)],
    );

    /// Draw a frame around an individual content item.
    ///
    /// This is called for each item before drawing its content.
    /// Default implementation does nothing (no item frames).
    fn draw_item_frame(
        &self,
        _cr: &Context,
        _config: &Self::Config,
        _x: f64,
        _y: f64,
        _w: f64,
        _h: f64,
    ) {
        // Default: no item frame
    }

    /// Run custom per-frame animation logic (e.g., scanlines, cursor blink).
    ///
    /// Called every animation frame. Returns true if a redraw is needed.
    /// Default implementation does nothing.
    fn animate_custom(&self, _config: &mut Self::Config, _elapsed: f64) -> bool {
        false
    }
}
