//! rg-sens-render: Cairo rendering functions for bars, arcs, graphs, etc.

pub mod arc_display;
pub mod background;
pub mod bar_display;
pub mod clock_display;
pub mod combo_traits;
pub mod core_bars_display;
pub mod cyberpunk_display;
pub mod graph_display;
pub mod industrial_display;
pub mod lcars_display;
pub mod material_display;
pub mod pango_text;
pub mod render_cache;
pub mod render_utils;
pub mod speedometer_display;
pub mod text_renderer;

// Themed frame renderers
pub mod art_deco_display;
pub mod art_nouveau_display;
pub mod fighter_hud_display;
pub mod retro_terminal_display;
pub mod steampunk_display;
pub mod synthwave_display;
