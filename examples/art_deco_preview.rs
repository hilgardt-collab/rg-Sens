//! Offscreen preview renderer for the Art Deco combo displayer.
//!
//! Renders a saved panel (default: `examples/Art Deco.json`) with the real
//! frame renderer and content pipeline into PNG files, plus one contact sheet
//! per style enum so every option can be judged side by side. No display
//! server is needed.
//!
//! ```text
//! cargo run --example art_deco_preview -- <out_dir> [panel.json]
//! ```
//!
//! A saved Retro Terminal or Fighter HUD panel (`examples/RetroTerminal.json`,
//! `examples/HUD.json`) renders too, as `current_full.png` only: enough to
//! compare a drawing change before and after.

use anyhow::{Context as _, Result};
use rg_sens::core::{global_registry, Displayer, DisplayerConfig, PanelData};
use rg_sens::displayers::{FrameRenderer, GenericComboDisplayerShared};
use rg_sens::ui::art_deco_display::ArtDecoRenderer;
use rg_sens::ui::fighter_hud_display::FighterHudRenderer;
use rg_sens::ui::retro_terminal_display::RetroTerminalRenderer;
use rg_sens_types::display_configs::art_deco::{
    ArtDecoFrameConfig, BackgroundPattern, BorderStyle, CornerStyle, DividerStyle, GroupFrameStyle,
    HeaderStyle,
};
use rg_sens_types::field::FieldType;
use serde_json::{json, Value};
use std::collections::HashMap;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let out = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "/tmp/art_deco_preview".to_string());
    let json_path = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "examples/Art Deco.json".to_string());
    std::fs::create_dir_all(&out)?;

    rg_sens::sources::register_all();
    rg_sens::displayers::register_all(false, false);

    let text = std::fs::read_to_string(&json_path).with_context(|| json_path.clone())?;
    let data: PanelData = serde_json::from_str(&text)?;
    let values = sample_values(&data)?;
    let frame = match &data.displayer_config {
        DisplayerConfig::ArtDeco(c) => c.to_frame(),
        DisplayerConfig::RetroTerminal(c) => {
            let surface = render_surface(RetroTerminalRenderer, &c.to_frame(), &values, 480, 1920)?;
            write_png(&surface, &format!("{}/current_full.png", out))?;
            println!("wrote preview to {}", out);
            return Ok(());
        }
        DisplayerConfig::FighterHud(c) => {
            let surface = render_surface(FighterHudRenderer, &c.to_frame(), &values, 480, 1920)?;
            write_png(&surface, &format!("{}/current_full.png", out))?;
            println!("wrote preview to {}", out);
            return Ok(());
        }
        other => anyhow::bail!(
            "panel is not Art Deco, Retro Terminal or Fighter HUD: {}",
            other.displayer_type()
        ),
    };

    // The panel as configured, at the user's size (30 x 120 cells of 16 px).
    render_png(
        &frame,
        &values,
        480,
        1920,
        &format!("{}/current_full.png", out),
    )?;

    // The same panel with the sizes the redesign is tuned for, to show what
    // a change of four sliders buys.
    let mut recommended = frame.clone();
    recommended.border_width = 4.0;
    recommended.corner_size = 36.0;
    recommended.divider_padding = 9.0;
    recommended.content_padding = 16.0;
    recommended.header_style = HeaderStyle::Sunburst;
    recommended.group_frame = GroupFrameStyle::Stepped;
    render_png(
        &recommended,
        &values,
        480,
        1920,
        &format!("{}/recommended_full.png", out),
    )?;

    // Contact sheets: a two-group version of the same panel, one tile per
    // enum variant, so each option can be judged at a glance.
    let mut small = frame.clone();
    small.group_count = 2;
    small.group_item_counts = frame.group_item_counts.iter().take(2).copied().collect();
    small.group_size_weights = frame.group_size_weights.iter().take(2).copied().collect();
    small.group_item_orientations = frame
        .group_item_orientations
        .iter()
        .take(2)
        .copied()
        .collect();

    sheet(
        &out,
        "borders",
        &values,
        [
            BorderStyle::Sunburst,
            BorderStyle::Chevron,
            BorderStyle::Stepped,
            BorderStyle::Geometric,
            BorderStyle::Ornate,
        ]
        .iter()
        .map(|s| {
            let mut f = small.clone();
            f.border_style = *s;
            (format!("{:?}", s), f)
        })
        .collect(),
    )?;
    sheet(
        &out,
        "corners",
        &values,
        [
            CornerStyle::Fan,
            CornerStyle::Ziggurat,
            CornerStyle::Diamond,
            CornerStyle::Bracket,
            CornerStyle::Hexagon,
            CornerStyle::Octagon,
            CornerStyle::Circle,
            CornerStyle::DoubleBracket,
            CornerStyle::GeometricStack,
            CornerStyle::None,
        ]
        .iter()
        .map(|s| {
            let mut f = small.clone();
            f.corner_style = *s;
            (format!("{:?}", s), f)
        })
        .collect(),
    )?;
    sheet(
        &out,
        "dividers",
        &values,
        [
            DividerStyle::Chevron,
            DividerStyle::DoubleLine,
            DividerStyle::Line,
            DividerStyle::Stepped,
            DividerStyle::DiamondCluster,
            DividerStyle::Crescent,
            DividerStyle::ArrowDiamond,
            DividerStyle::CircleChain,
            DividerStyle::CrossedLines,
            DividerStyle::FleurDeLis,
            DividerStyle::Heartbeat,
            DividerStyle::DiamondGrid,
            DividerStyle::None,
            DividerStyle::Zigzag,
            DividerStyle::Fan,
            DividerStyle::Pyramid,
        ]
        .iter()
        .map(|s| {
            let mut f = small.clone();
            f.divider_style = *s;
            (format!("{:?}", s), f)
        })
        .collect(),
    )?;
    sheet(
        &out,
        "headers",
        &values,
        [
            HeaderStyle::Centered,
            HeaderStyle::Banner,
            HeaderStyle::Stepped,
            HeaderStyle::None,
            HeaderStyle::Sunburst,
        ]
        .iter()
        .map(|s| {
            let mut f = small.clone();
            f.header_style = *s;
            f.show_header = true;
            (format!("{:?}", s), f)
        })
        .collect(),
    )?;
    sheet(
        &out,
        "patterns",
        &values,
        [
            BackgroundPattern::Solid,
            BackgroundPattern::VerticalLines,
            BackgroundPattern::DiamondGrid,
            BackgroundPattern::Sunburst,
            BackgroundPattern::Chevrons,
            BackgroundPattern::Lattice,
        ]
        .iter()
        .map(|s| {
            let mut f = small.clone();
            f.background_pattern = *s;
            (format!("{:?}", s), f)
        })
        .collect(),
    )?;

    sheet(
        &out,
        "frames",
        &values,
        [
            GroupFrameStyle::None,
            GroupFrameStyle::Panel,
            GroupFrameStyle::Stepped,
        ]
        .iter()
        .map(|s| {
            let mut f = small.clone();
            f.group_frame = *s;
            f.divider_padding = 9.0;
            (format!("{:?}", s), f)
        })
        .collect(),
    )?;

    println!("wrote previews to {}", out);
    Ok(())
}

/// Real values where the sources work headlessly (clock, cpu, memory),
/// plausible fakes for everything else so every item renders something.
fn sample_values(data: &PanelData) -> Result<HashMap<String, Value>> {
    let mut source = global_registry().create_source(data.source_config.source_type())?;
    source.configure_typed(&data.source_config)?;
    let _ = source.update();
    let mut values = source.get_values();
    let mut seed = 0x9E37_79B9u64;
    for field in source.fields() {
        let usable = match values.get(&field.id) {
            Some(Value::Number(n)) => n.as_f64().map(|v| v != 0.0).unwrap_or(false),
            Some(Value::String(s)) => !s.is_empty() && s != "N/A",
            Some(v) => !v.is_null(),
            None => false,
        };
        if !usable {
            values.insert(field.id.clone(), fake_value(&field.field_type, &mut seed));
        }
    }
    Ok(values)
}

fn fake_value(field_type: &FieldType, seed: &mut u64) -> Value {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    match field_type {
        FieldType::Numerical | FieldType::Percentage => {
            json!(12.0 + (*seed % 7600) as f64 / 100.0)
        }
        _ => json!("Sample"),
    }
}

fn render_surface<R: FrameRenderer>(
    renderer: R,
    frame: &R::Config,
    values: &HashMap<String, Value>,
    w: i32,
    h: i32,
) -> Result<cairo::ImageSurface> {
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, w, h)?;
    {
        let cr = cairo::Context::new(&surface)?;
        // Window background behind the panel.
        cr.set_source_rgb(0.05, 0.05, 0.06);
        cr.paint()?;
        let mut displayer = GenericComboDisplayerShared::new(renderer);
        displayer.set_config(frame.clone());
        displayer.update_data(values);
        displayer.render_to_context(&cr, w, h);
    }
    Ok(surface)
}

fn write_png(surface: &cairo::ImageSurface, path: &str) -> Result<()> {
    let mut file = std::fs::File::create(path)?;
    surface.write_to_png(&mut file)?;
    Ok(())
}

fn render_png(
    frame: &ArtDecoFrameConfig,
    values: &HashMap<String, Value>,
    w: i32,
    h: i32,
    path: &str,
) -> Result<()> {
    let surface = render_surface(ArtDecoRenderer, frame, values, w, h)?;
    write_png(&surface, path)
}

/// One PNG with a labelled tile per variant.
fn sheet(
    out: &str,
    name: &str,
    values: &HashMap<String, Value>,
    variants: Vec<(String, ArtDecoFrameConfig)>,
) -> Result<()> {
    const TILE_W: i32 = 480;
    const TILE_H: i32 = 420;
    const LABEL_H: i32 = 28;
    const GAP: i32 = 12;
    let cols = 3.min(variants.len() as i32).max(1);
    let rows = (variants.len() as i32 + cols - 1) / cols;
    let sheet_w = cols * TILE_W + (cols + 1) * GAP;
    let sheet_h = rows * (TILE_H + LABEL_H) + (rows + 1) * GAP;
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, sheet_w, sheet_h)?;
    {
        let cr = cairo::Context::new(&surface)?;
        cr.set_source_rgb(0.16, 0.16, 0.18);
        cr.paint()?;
        cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        cr.set_font_size(16.0);
        for (i, (label, frame)) in variants.iter().enumerate() {
            let col = i as i32 % cols;
            let row = i as i32 / cols;
            let x = GAP + col * (TILE_W + GAP);
            let y = GAP + row * (TILE_H + LABEL_H + GAP);
            cr.set_source_rgb(0.9, 0.9, 0.9);
            cr.move_to(x as f64 + 4.0, y as f64 + 19.0);
            cr.show_text(label)?;
            let tile = render_surface(ArtDecoRenderer, frame, values, TILE_W, TILE_H)?;
            cr.set_source_surface(&tile, x as f64, (y + LABEL_H) as f64)?;
            cr.paint()?;
        }
    }
    let path = format!("{}/{}.png", out, name);
    let mut file = std::fs::File::create(&path)?;
    surface.write_to_png(&mut file)?;
    Ok(())
}
