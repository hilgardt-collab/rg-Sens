//! The optional title bar across the top of a combo group's area.
//!
//! Every combo frame draws it the same way, inside the group's rectangle and
//! before the group's items, so the frame's dividers and layout cache are
//! untouched; the caller hands the remaining height to the items.

use gtk4::cairo;

use crate::pango_text::{pango_show_text, pango_text_extents};
use rg_sens_types::combo::{GroupTitleConfig, TitleAlign};
use rg_sens_types::theme::ComboThemeConfig;

/// Horizontal padding between the bar's edge and the text
const PADDING: f64 = 6.0;

/// Draw `title` across the top of the group rectangle and return the height
/// it used (0 when disabled or the text is empty). The bar is at most half
/// the group's height so the items always keep room.
pub fn render_group_title(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    title: &GroupTitleConfig,
    theme: &ComboThemeConfig,
) -> Result<f64, cairo::Error> {
    if !title.enabled || title.text.trim().is_empty() || w <= 0.0 || h <= 0.0 {
        return Ok(0.0);
    }
    let band = title.height.clamp(0.0, h / 2.0);
    if band <= 0.0 {
        return Ok(0.0);
    }
    cr.save()?;
    cr.rectangle(x, y, w, band);
    cr.clip();

    if title.show_background {
        let bg = title.background.resolve(theme);
        cr.set_source_rgba(bg.r, bg.g, bg.b, bg.a);
        cr.rectangle(x, y, w, band);
        cr.fill()?;
    }

    let (family, size) = title.font.resolve(theme);
    // Keep the text inside the bar whatever size the font asks for
    let size = size.min(band * 0.8).max(1.0);
    let weight = if title.bold {
        cairo::FontWeight::Bold
    } else {
        cairo::FontWeight::Normal
    };
    let extents = pango_text_extents(
        cr,
        &title.text,
        &family,
        cairo::FontSlant::Normal,
        weight,
        size,
    );
    let text_w = extents.width();
    let text_h = extents.height().max(size);
    let text_x = match title.align {
        TitleAlign::Left => x + PADDING,
        TitleAlign::Center => x + (w - text_w) / 2.0,
        TitleAlign::Right => x + w - text_w - PADDING,
    };
    // `pango_show_text` draws from the current point as the baseline
    let baseline = y + band / 2.0 + text_h / 2.0;
    let color = title.color.resolve(theme);
    cr.set_source_rgba(color.r, color.g, color.b, color.a);
    cr.move_to(text_x, baseline);
    pango_show_text(
        cr,
        &title.text,
        &family,
        cairo::FontSlant::Normal,
        weight,
        size,
    );
    cr.restore()?;
    Ok(band)
}
