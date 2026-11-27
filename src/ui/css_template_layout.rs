//! Custom layouts for CSS templates: groups and widgets chosen in the dialog.
//!
//! A template that wants to be re-arranged declares its building blocks as
//! inert `<template>` elements and marks where its own groups sit:
//!
//! ```html
//! <!--rg-groups 0-71 90-->            the template's own groups (the default);
//!   ...                               the numbers are the placeholders they use
//! <!--/rg-groups-->
//!
//! <template data-rg-group="section" data-rg-label="Section">
//!   <section><h2>{{title}}</h2><!--rg-items--></section>
//! </template>
//! <template data-rg-widget="bar" data-rg-size="small" data-rg-label="Bar">
//!   <div class="stat" data-need="{{value}}"><i data-bar="{{value}}" data-max="{{max}}"></i></div>
//! </template>
//! ```
//!
//! With a custom layout on, the marked region is replaced, before the page
//! loads, by one group prototype per group of the panel's source and one
//! widget prototype per slot. A `{{field}}` token in a widget becomes a
//! placeholder number of its own (from [`FIRST_INDEX`] up) bound to that
//! field of the widget's slot, so the template's script reads generated
//! widgets exactly as it reads hand-written ones. `{{field:.1}}` asks for a
//! number format, `{{slot}}` is the slot's name (for ids).
//!
//! Nothing here runs for a template without prototypes or a panel left on
//! the template's own layout: that document is untouched.

use once_cell::sync::Lazy;
use regex::Regex;
use rg_sens_types::display_configs::css_template::{
    CssLayoutConfig, CssWidgetChoice, PlaceholderMapping, CSS_LAYOUT_MAX_ITEMS,
};
use std::collections::HashMap;

/// Generated widgets take their placeholder numbers from here up, clear of
/// anything a template numbers by hand.
pub const FIRST_INDEX: u32 = 1000;

/// The widget kind of a slot that is not drawn in any group: a sensor that
/// only feeds something outside the groups (a footer's uptime).
pub const HIDDEN: &str = "none";

pub const SIZES: [(&str, &str); 3] = [
    ("small", "Small (1 x 1)"),
    ("medium", "Medium (2 x 2)"),
    ("large", "Large (4 x 4)"),
];

/// The most cells a widget can take either way.
pub const MAX_SPAN: u32 = 8;

/// The size that stands for a square block of `span` cells a side.
pub fn size_for(span: u32) -> Option<&'static str> {
    SIZES
        .iter()
        .map(|(id, _)| *id)
        .find(|id| span_of(id) == span)
}

/// The cells a size takes each way on its group's grid.
pub fn span_of(size: &str) -> u32 {
    match size {
        "large" => 4,
        "medium" => 2,
        _ => 1,
    }
}

static TEMPLATE_TAG: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?s)<template\s([^>]*)>(.*?)</template>").expect("Invalid template regex")
});
static RG_ATTR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"data-rg-([a-z-]+)\s*=\s*"([^"]*)""#).expect("Invalid attr regex"));
static GROUPS_REGION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?s)<!--\s*rg-groups\b([^>]*?)-->(.*?)<!--\s*/rg-groups\s*-->")
        .expect("Invalid region regex")
});
static ITEMS_MARK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<!--\s*rg-items\s*-->").expect("Invalid items regex"));
static TOKEN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\{\{([a-z_][a-z0-9_]*)(?::(\.\d))?\}\}").expect("Invalid token regex")
});
static GROUP_SIZE_META: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<meta\s+name="rg-group-size"\s+content="(\d+)x(\d+)(?:\+(\d+))?"\s*/?>"#)
        .expect("Invalid group size regex")
});
static LAYOUTS_META: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<meta\s+name="rg-layouts"\s+content="[^"]*"\s*/?>"#)
        .expect("Invalid layouts regex")
});
static DESIGN_META: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<meta\s+name="rg-design-size"\s+content="[^"]*"\s*/?>"#)
        .expect("Invalid design size regex")
});

/// A widget prototype.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetProto {
    pub kind: String,
    pub label: String,
    /// `large`, `medium` or `small`.
    pub size: String,
    /// The sources it is made for; empty takes any.
    pub sources: Vec<String>,
    /// The sources it is the first choice for while staying on offer for
    /// every other (`data-rg-default-for`): a vessel for memory and disk.
    pub default_for: Vec<String>,
    /// The group prototype a group of these wants (`data-rg-group`).
    pub group: String,
    /// The size it is drawn at, in the design's pixels (`data-rg-width`,
    /// `data-rg-height`); it is scaled from that to the cells it is given.
    natural: Option<(u32, u32)>,
    /// The colours it can be drawn in (`data-rg-colors`): what `{{color}}`
    /// in its markup may become, usually class names of the template.
    pub colors: Vec<String>,
    html: String,
}

/// A group prototype.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupProto {
    pub id: String,
    pub label: String,
    html: String,
}

/// What a template offers for custom layouts.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayoutSpec {
    pub widgets: Vec<WidgetProto>,
    pub groups: Vec<GroupProto>,
    /// The placeholders the template's own groups use; a custom layout
    /// removes those groups, so their bindings no longer apply.
    pub own_indices: Vec<u32>,
    /// `rg-group-size`: the room one group needs, and what the page needs
    /// outside its groups (height).
    group_size: Option<(u32, u32, u32)>,
    has_region: bool,
}

impl LayoutSpec {
    /// Whether the template can be re-arranged at all.
    pub fn usable(&self) -> bool {
        self.has_region && !self.widgets.is_empty() && !self.groups.is_empty()
    }

    /// The widget kinds, in template order, as `(kind, label)`.
    pub fn kinds(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        for w in &self.widgets {
            if !out.iter().any(|(k, _)| *k == w.kind) {
                out.push((w.kind.clone(), w.label.clone()));
            }
        }
        out
    }

    fn made_for(widget: &WidgetProto, source: &str) -> bool {
        widget
            .sources
            .iter()
            .any(|s| crate::ui::css_template_display::source_matches(source, s))
    }

    /// The kinds offered for a slot fed by `source`: first the ones made for
    /// that source (`data-rg-sources`), then the ones made for no source in
    /// particular. A clock has no reading to put on a dial, so it is offered
    /// only its own widgets when the template has any (as a combo item's
    /// "Display as" list does).
    pub fn kinds_for(&self, source: &str) -> Vec<(String, String)> {
        let has = |kind: &str, test: &dyn Fn(&WidgetProto) -> bool| {
            self.widgets.iter().any(|w| w.kind == kind && test(w))
        };
        let kinds = self.kinds();
        let mut out: Vec<(String, String)> = kinds
            .iter()
            .filter(|(kind, _)| has(kind, &|w| Self::made_for(w, source)))
            .cloned()
            .collect();
        if source != "clock" || out.is_empty() {
            for (kind, label) in kinds {
                if has(&kind, &|w| w.sources.is_empty()) && !out.iter().any(|(k, _)| *k == kind) {
                    out.push((kind, label));
                }
            }
        }
        out.push((HIDDEN.to_string(), "Not shown".to_string()));
        out
    }

    /// The size a choice is drawn in: its own, else the first one the
    /// template drew its kind in.
    pub fn size_of(&self, choice: &CssWidgetChoice) -> &'static str {
        let known = |size: &str| SIZES.iter().map(|(id, _)| *id).find(|id| *id == size);
        known(&choice.size)
            .or_else(|| {
                self.widgets
                    .iter()
                    .find(|w| w.kind == choice.kind)
                    .and_then(|w| known(&w.size))
            })
            .unwrap_or("medium")
    }

    /// The colours the widget a choice names can be drawn in.
    pub fn colors_of(&self, choice: &CssWidgetChoice) -> &[String] {
        self.widget(Some(choice)).map_or(&[], |w| &w.colors)
    }

    /// The block of cells a choice takes, as `(columns, rows)`: the one the
    /// user gave it, else the square of its size.
    pub fn block_of(&self, choice: &CssWidgetChoice) -> (u32, u32) {
        match choice.span {
            Some((w, h)) => (w.clamp(1, MAX_SPAN), h.clamp(1, MAX_SPAN)),
            None => {
                let side = span_of(self.size_of(choice));
                (side, side)
            }
        }
    }

    /// The prototype a choice names (every prototype scales to its cells):
    /// for a square size, its kind as drawn for that size or the nearest
    /// one; for a block the user shaped, the drawing of its kind whose
    /// proportions fill that block best (a wide bar for 4x1, a compact one
    /// for 1x1). Else the template's first widget.
    fn widget(&self, choice: Option<&CssWidgetChoice>) -> Option<&WidgetProto> {
        self.widget_in(choice, 1.0)
    }

    /// `widget`, for cells `cell_shape` times as wide as they are high: the
    /// grid is stretched over the group's box, so its cells are square only
    /// when the grid has the box's proportions.
    fn widget_in(&self, choice: Option<&CssWidgetChoice>, cell_shape: f64) -> Option<&WidgetProto> {
        choice
            .and_then(|c| {
                let of_kind = self.widgets.iter().filter(|w| w.kind == c.kind);
                let wanted = span_of(self.size_of(c)) as i64;
                let off = |w: &WidgetProto| (span_of(&w.size) as i64 - wanted).abs();
                if c.span.is_none() {
                    return of_kind.min_by_key(|w| off(w));
                }
                let (bw, bh) = self.block_of(c);
                let (bw, bh) = (bw as f64 * cell_shape, bh as f64);
                // The share of the block the drawing covers once scaled in.
                let fill = |w: &WidgetProto| match w.natural {
                    Some((nw, nh)) if nw > 0 && nh > 0 => {
                        let (nw, nh) = (nw as f64, nh as f64);
                        let k = (bw / nw).min(bh / nh);
                        nw * nh * k * k / (bw * bh)
                    }
                    _ => 0.5,
                };
                of_kind.min_by(|a, b| fill(b).total_cmp(&fill(a)).then(off(a).cmp(&off(b))))
            })
            .or_else(|| self.widgets.first())
    }

    /// The widget a slot gets when the user has not chosen one: called when
    /// a custom layout is first turned on, when a slot is added and when a
    /// slot's source changes to one its widget is not offered for.
    ///
    /// `source` is the slot's source id (`cpu`), `metric` what it measures
    /// in the source's own words (`Temperature`, `Usage`, `MemoryUsed`; may
    /// be empty) and `position` the slot's place in its group (0 first).
    pub fn default_widget(&self, source: &str, metric: &str, position: usize) -> CssWidgetChoice {
        let mut offered = self.kinds_for(source);
        offered.retain(|(kind, _)| kind != HIDDEN);
        let own = self.widgets.iter().any(|w| Self::made_for(w, source));
        // TODO(human): this is the policy that decides what a fresh custom
        // layout looks like; change it to taste. As written: a source the
        // template has a widget for gets that widget at its own size (a fan
        // its pinwheel, a clock its face); otherwise the first widget of a
        // group and every temperature get the template's first kind (its
        // showpiece, usually a dial) at medium, 2 x 2, and the rest its
        // second (usually a bar) as a 2 x 1 block. Unplaced, that reads as
        // most designs do: the dial on the left, the bars stacked beside it.
        let showpiece = position == 0 || metric.to_lowercase().contains("temp");
        let pick = |at: usize| {
            offered
                .get(at)
                .or(offered.first())
                .map(|(kind, _)| kind.clone())
                .unwrap_or_default()
        };
        // A widget that names this source as the one it is meant for.
        let meant = self.widgets.iter().find(|w| {
            offered.iter().any(|(kind, _)| *kind == w.kind)
                && w.default_for
                    .iter()
                    .any(|s| crate::ui::css_template_display::source_matches(source, s))
        });
        if let Some(widget) = meant.filter(|_| !own) {
            return CssWidgetChoice {
                kind: widget.kind.clone(),
                size: widget.size.clone(),
                ..Default::default()
            };
        }
        if own {
            CssWidgetChoice {
                kind: pick(0),
                ..Default::default()
            }
        } else if showpiece {
            CssWidgetChoice {
                kind: pick(0),
                size: "medium".to_string(),
                ..Default::default()
            }
        } else {
            CssWidgetChoice {
                kind: pick(1),
                span: Some((2, 1)),
                ..Default::default()
            }
        }
    }

    /// `choice` if the slot's source is offered that kind, else the default.
    pub fn coerce(
        &self,
        choice: Option<&CssWidgetChoice>,
        source: &str,
        metric: &str,
        position: usize,
    ) -> CssWidgetChoice {
        match choice {
            Some(c) if self.kinds_for(source).iter().any(|(k, _)| *k == c.kind) => {
                let size_ok = SIZES.iter().any(|(s, _)| *s == c.size);
                let mut kept = CssWidgetChoice {
                    kind: c.kind.clone(),
                    size: if size_ok {
                        c.size.clone()
                    } else {
                        String::new()
                    },
                    color: String::new(),
                    pos: c.pos,
                    span: c
                        .span
                        .map(|(w, h)| (w.clamp(1, MAX_SPAN), h.clamp(1, MAX_SPAN))),
                };
                if self.colors_of(&kept).contains(&c.color) {
                    kept.color = c.color.clone();
                }
                kept
            }
            _ => self.default_widget(source, metric, position),
        }
    }
}

/// The template as it is on disk now, else the HTML embedded in the config.
pub fn read_template(
    config: &rg_sens_types::display_configs::css_template::CssTemplateDisplayConfig,
) -> Option<String> {
    if !config.html_path.as_os_str().is_empty() {
        if let Ok(html) = std::fs::read_to_string(&config.html_path) {
            return Some(html);
        }
    }
    config.embedded_html.clone()
}

/// Give every slot of the panel a widget its source is offered, keeping the
/// user's choices that still fit. The dialog runs this before it shows or
/// applies a custom layout, so the page and the dialog agree.
pub fn coerce_layout(
    layout: &mut CssLayoutConfig,
    spec: &LayoutSpec,
    slots: &[crate::ui::css_template_display::SlotKind],
    mappings: &[PlaceholderMapping],
) {
    use rg_sens_types::display_configs::css_template::parse_slot;
    // A sensor the template shows only outside its groups (the footer's
    // uptime) stays out of the groups until the user gives it a widget.
    let only_outside = |slot: &str| {
        let mut bound = mappings.iter().filter(|m| m.slot_prefix == slot).peekable();
        bound.peek().is_some() && bound.all(|m| !spec.own_indices.contains(&m.index))
    };
    let mut counts: Vec<u32> = Vec::new();
    for slot in slots {
        let Some((group, position)) = parse_slot(&slot.prefix) else {
            continue;
        };
        if counts.len() <= group {
            counts.resize(group + 1, 0);
        }
        counts[group] = counts[group].max(position as u32 + 1);
        let chosen = layout.widgets.get(&slot.prefix);
        let choice = if chosen.is_none() && only_outside(&slot.prefix) {
            CssWidgetChoice {
                kind: HIDDEN.to_string(),
                ..Default::default()
            }
        } else {
            spec.coerce(chosen, &slot.source, &slot.metric, position)
        };
        layout.widgets.insert(slot.prefix.clone(), choice);
    }
    layout.sync(counts);
}

/// A template as Scan Placeholders would set a new panel up for it.
pub struct Scanned {
    /// The sensors the template asks for, one slot each.
    pub slots: Vec<crate::ui::css_template_display::SlotKind>,
    /// Its placeholders bound to those slots.
    pub mappings: Vec<PlaceholderMapping>,
    /// The custom layout the dialog would start from (off, and empty, for
    /// a template without prototypes).
    pub layout: CssLayoutConfig,
}

/// Scan `html` the way the dialog does for a fresh panel: its sensors as
/// slots, its placeholders bound, and the custom layout it starts with.
/// The example panels' tests and the render harness share this.
pub fn scan_template(html: &str) -> Scanned {
    use crate::ui::config_tree::util::{add_wanted_slots, combo_slot_kinds};
    use crate::ui::css_template_display::{
        auto_map, detect_placeholders, extract_placeholder_defaults, wanted_slots,
    };
    let found = detect_placeholders(html);
    let defaults = extract_placeholder_defaults(html);
    let mut source = crate::sources::ComboSourceConfig::default();
    add_wanted_slots(&mut source, &wanted_slots(&found, &defaults));
    let slots = combo_slot_kinds(&source);
    let mappings = auto_map(&found, &defaults, &HashMap::new(), &slots);
    let spec = layout_spec(html);
    let mut layout = CssLayoutConfig::default();
    if spec.usable() {
        layout.custom = true;
        coerce_layout(&mut layout, &spec, &slots, &mappings);
    }
    Scanned {
        slots,
        mappings,
        layout,
    }
}

fn attrs(text: &str) -> HashMap<String, String> {
    RG_ATTR
        .captures_iter(text)
        .map(|c| (c[1].to_string(), c[2].to_string()))
        .collect()
}

/// `0-71 90` as the numbers it lists.
fn index_list(text: &str) -> Vec<u32> {
    let mut out = Vec::new();
    for part in text.split(|c: char| c.is_whitespace() || c == ',') {
        match part.split_once('-') {
            Some((a, b)) => {
                if let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>()) {
                    if a <= b && b - a < 10_000 {
                        out.extend(a..=b);
                    }
                }
            }
            None => out.extend(part.parse::<u32>().ok()),
        }
    }
    out
}

/// Read a template's prototypes (see the module documentation).
pub fn layout_spec(html: &str) -> LayoutSpec {
    let mut spec = LayoutSpec::default();
    for cap in TEMPLATE_TAG.captures_iter(html) {
        let a = attrs(&cap[1]);
        let get = |key: &str| a.get(key).cloned().unwrap_or_default();
        let body = cap[2].trim().to_string();
        if let Some(kind) = a.get("widget").filter(|k| !k.is_empty()) {
            let size = Some(get("size"))
                .filter(|s| SIZES.iter().any(|(id, _)| id == s))
                .unwrap_or_else(|| "medium".to_string());
            if spec
                .widgets
                .iter()
                .any(|w| w.kind == *kind && w.size == size)
            {
                continue;
            }
            spec.widgets.push(WidgetProto {
                kind: kind.clone(),
                label: Some(get("label"))
                    .filter(|l| !l.is_empty())
                    .unwrap_or_else(|| kind.clone()),
                size,
                sources: get("sources")
                    .split_whitespace()
                    .map(str::to_string)
                    .collect(),
                default_for: get("default-for")
                    .split_whitespace()
                    .map(str::to_string)
                    .collect(),
                group: get("group"),
                natural: get("width").parse().ok().zip(get("height").parse().ok()),
                colors: get("colors")
                    .split_whitespace()
                    .map(str::to_string)
                    .collect(),
                html: body,
            });
        } else if let Some(id) = a.get("group").filter(|k| !k.is_empty()) {
            if spec.groups.iter().any(|g| g.id == *id) || !ITEMS_MARK.is_match(&body) {
                continue;
            }
            spec.groups.push(GroupProto {
                id: id.clone(),
                label: Some(get("label"))
                    .filter(|l| !l.is_empty())
                    .unwrap_or_else(|| id.clone()),
                html: body,
            });
        }
    }
    if let Some(cap) = GROUPS_REGION.captures(html) {
        spec.has_region = true;
        spec.own_indices = index_list(&cap[1]);
    }
    spec.group_size = GROUP_SIZE_META.captures(html).and_then(|c| {
        Some((
            c[1].parse().ok()?,
            c[2].parse().ok()?,
            c.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
        ))
    });
    spec
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Rules for generated groups: a grid of cells that share the group's box,
/// each widget centred in the cells it spans and scaled to them by the
/// bridge script. The widget is out of flow, so the cells are sized by the
/// group alone, never by what they hold.
/// `:where()` keeps them at zero specificity, so anything a template says
/// about these classes wins.
const LAYOUT_CSS: &str = "<style>\
:where(.rg-items){display:grid;gap:8px;flex:1;min-width:0;min-height:0}\
:where(.rg-item){min-width:0;min-height:0;overflow:hidden;position:relative}\
:where(.rg-fit){position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);transform-origin:center}\
</style>";

/// Where a group's widgets sit on its grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// `(column, row, columns, rows)` of each widget, in the order given.
    pub cells: Vec<(u32, u32, u32, u32)>,
    pub cols: u32,
    pub rows: u32,
}

/// Whether two blocks, each `(column, row, columns, rows)`, share a cell.
pub fn overlap(a: (u32, u32, u32, u32), b: (u32, u32, u32, u32)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// Place a group's widgets, given each one's block (`(columns, rows)`) and
/// the cell the user put it in. A widget without a cell, or whose cell is
/// taken by an earlier one, goes to the first free spot, row by row, on a
/// grid as wide as the widest thing placed (at least four cells: one large
/// widget). The result starts at column and row 0.
pub fn place(widgets: &[((u32, u32), Option<(u32, u32)>)]) -> Placement {
    type Block = (u32, u32, u32, u32);
    let free = |taken: &[Block], block: Block| taken.iter().all(|t| !overlap(*t, block));
    let mut taken: Vec<Block> = Vec::new();
    let mut cells: Vec<Option<Block>> = vec![None; widgets.len()];
    for (i, ((w, h), pos)) in widgets.iter().enumerate() {
        if let Some((c, r)) = pos {
            let block = (*c, *r, *w, *h);
            if free(&taken, block) {
                taken.push(block);
                cells[i] = Some(block);
            }
        }
    }
    let width = taken.iter().map(|t| t.0 + t.2).max().unwrap_or(0).max(4);
    for (i, ((w, h), _)) in widgets.iter().enumerate() {
        if cells[i].is_some() {
            continue;
        }
        let block = (0..)
            .flat_map(|r| (0..=width.saturating_sub(*w)).map(move |c| (c, r, *w, *h)))
            .find(|block| free(&taken, *block))
            .unwrap_or((0, 0, *w, *h));
        taken.push(block);
        cells[i] = Some(block);
    }
    let mut cells: Vec<Block> = cells.into_iter().flatten().collect();
    let (left, top) = (
        cells.iter().map(|c| c.0).min().unwrap_or(0),
        cells.iter().map(|c| c.1).min().unwrap_or(0),
    );
    for cell in &mut cells {
        cell.0 -= left;
        cell.1 -= top;
    }
    Placement {
        cols: cells.iter().map(|c| c.0 + c.2).max().unwrap_or(1),
        rows: cells.iter().map(|c| c.1 + c.3).max().unwrap_or(1),
        cells,
    }
}

/// The arrangements a page of `groups` groups can take, as `rg-layouts`
/// content: one per useful column count, narrowest first.
fn layouts_for(groups: usize, (w, h, extra): (u32, u32, u32)) -> String {
    let groups = groups.max(1) as u32;
    let mut out = Vec::new();
    let mut last_rows = 0;
    for cols in 1..=groups {
        let rows = groups.div_ceil(cols);
        if rows != last_rows {
            out.push(format!("{}x{}", w * cols, h * rows + extra));
            last_rows = rows;
        }
    }
    out.join(" ")
}

/// A template with its custom layout built in.
pub struct Expanded {
    pub html: String,
    /// The bindings of the generated widgets.
    pub mappings: Vec<PlaceholderMapping>,
}

/// Build `layout` into `html`. `None` when the layout is off or the
/// template cannot be re-arranged; the caller then uses `html` as it is.
pub fn expand_layout(html: &str, layout: &CssLayoutConfig) -> Option<Expanded> {
    if !layout.custom {
        return None;
    }
    let spec = layout_spec(html);
    if !spec.usable() {
        return None;
    }
    let mut mappings: Vec<PlaceholderMapping> = Vec::new();
    let mut next = FIRST_INDEX;
    let mut groups_html = String::from(LAYOUT_CSS);
    let mut shown_groups = 0;
    for (g, count) in layout.item_counts.iter().enumerate() {
        let group = layout.groups.get(g).cloned().unwrap_or_default();
        // The widgets that are drawn, then where each goes.
        let mut drawn: Vec<(String, &WidgetProto, Option<&CssWidgetChoice>)> = Vec::new();
        for i in 0..*count as usize {
            let slot = format!("group{}_{}", g + 1, i + 1);
            let choice = layout.widgets.get(&slot);
            if choice.is_some_and(|c| c.kind == HIDDEN) || drawn.len() == CSS_LAYOUT_MAX_ITEMS {
                continue;
            }
            if let Some(proto) = spec.widget(choice) {
                drawn.push((slot, proto, choice));
            }
        }
        // A group whose sensors are all "Not shown" is not a group here.
        if drawn.is_empty() {
            continue;
        }
        shown_groups += 1;
        let spans: Vec<((u32, u32), Option<(u32, u32)>)> = drawn
            .iter()
            .map(|(_, proto, choice)| match choice {
                Some(c) => (spec.block_of(c), c.pos),
                None => ((span_of(&proto.size), span_of(&proto.size)), None),
            })
            .collect();
        let placement = place(&spans);
        // Now that the grid is known, so is the shape of its cells: a block
        // the user shaped gets the drawing that suits the room it really has.
        let cell_shape = spec.group_size.map_or(1.0, |(w, h, _)| {
            (w as f64 / placement.cols as f64) / (h as f64 / placement.rows as f64)
        });
        for (_, proto, choice) in &mut drawn {
            if choice.is_some_and(|c| c.span.is_some()) {
                if let Some(better) = spec.widget_in(*choice, cell_shape) {
                    *proto = better;
                }
            }
        }
        let mut items = String::new();
        for (at, (slot, proto, choice)) in drawn.iter().enumerate() {
            // Its own colour, else the template's colours in turn.
            let color = choice
                .map(|c| c.color.as_str())
                .filter(|c| proto.colors.iter().any(|p| p == c))
                .or_else(|| {
                    (!proto.colors.is_empty())
                        .then(|| proto.colors[at % proto.colors.len()].as_str())
                })
                .unwrap_or("");
            // One placeholder per (field, format) the widget reads.
            let mut taken: HashMap<(String, Option<String>), u32> = HashMap::new();
            let body = TOKEN.replace_all(&proto.html, |caps: &regex::Captures| {
                let field = &caps[1];
                if field == "slot" {
                    return slot.clone();
                }
                if field == "color" {
                    return color.to_string();
                }
                let format = caps.get(2).map(|m| format!("{{:{}}}", m.as_str()));
                let index = *taken
                    .entry((field.to_string(), format.clone()))
                    .or_insert_with(|| {
                        let index = next;
                        next += 1;
                        mappings.push(PlaceholderMapping {
                            index,
                            slot_prefix: slot.clone(),
                            field: field.to_string(),
                            format,
                        });
                        index
                    });
                index.to_string()
            });
            let (col, row, wide, high) = placement.cells[at];
            // The size its block stands for; a shaped block has none, so
            // it is named after the drawing it uses.
            let size = size_for(wide)
                .filter(|_| wide == high)
                .unwrap_or(&proto.size);
            let natural = proto
                .natural
                .map(|(w, h)| format!(r#" style="width:{w}px;height:{h}px""#))
                .unwrap_or_default();
            items.push_str(&format!(
                r#"<div class="rg-item rg-size-{size} rg-widget-{kind}" data-rg-slot="{slot}" style="grid-column:{c}/span {wide};grid-row:{r}/span {high}"><div class="rg-fit"{natural}>{body}</div></div>"#,
                kind = proto.kind,
                c = col + 1,
                r = row + 1,
            ));
        }
        let container = format!(
            r#"<div class="rg-items" style="grid-template-columns:repeat({},minmax(0,1fr));grid-template-rows:repeat({},minmax(0,1fr))">{}</div>"#,
            placement.cols, placement.rows, items
        );
        let proto = spec
            .groups
            .iter()
            .find(|p| p.id == group.style)
            .or_else(|| spec.groups.iter().find(|p| p.id == drawn[0].1.group))
            .or_else(|| spec.groups.first())?;
        let title = if group.title.is_empty() {
            format!("Group {}", g + 1)
        } else {
            group.title.clone()
        };
        let shell = ITEMS_MARK.replace(&proto.html, regex::NoExpand(&container));
        // `{{index}}` counts the groups that are drawn, from 1; `{{parity}}`
        // is `odd` or `even`, for designs whose groups alternate.
        groups_html.push_str(
            &shell
                .replace("{{title}}", &escape(&title))
                .replace("{{index}}", &shown_groups.to_string())
                .replace(
                    "{{parity}}",
                    if shown_groups % 2 == 1 { "odd" } else { "even" },
                ),
        );
    }
    let mut out = GROUPS_REGION
        .replace(html, regex::NoExpand(&groups_html))
        .into_owned();
    // The page is fitted to the panel by the room its groups need; that
    // follows the number of groups now.
    if let Some(size) = spec.group_size {
        let layouts = layouts_for(shown_groups, size);
        let first = layouts.split(' ').next().unwrap_or_default().to_string();
        let meta = format!(r#"<meta name="rg-layouts" content="{layouts}">"#);
        out = if LAYOUTS_META.is_match(&out) {
            LAYOUTS_META
                .replace(&out, regex::NoExpand(&meta))
                .into_owned()
        } else {
            DESIGN_META
                .replace(&out, |c: &regex::Captures| format!("{}{}", &c[0], meta))
                .into_owned()
        };
        out = DESIGN_META
            .replace(
                &out,
                regex::NoExpand(&format!(
                    r#"<meta name="rg-design-size" content="{first}">"#
                )),
            )
            .into_owned();
    }
    Some(Expanded {
        html: out,
        mappings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rg_sens_types::display_configs::css_template::CssGroupLayout;

    const PAGE: &str = r#"<html><head>
<meta name="rg-design-size" content="480x960">
<meta name="rg-layouts" content="480x960 960x480">
<meta name="rg-group-size" content="480x300+60">
</head><body>
<div class="groups">
<!--rg-groups 0-7-->
<section><b data-num="1"></b></section>
<!--/rg-groups-->
</div>
<i data-uptime="73"></i>
<template data-rg-group="section" data-rg-label="Section"><section><h2>{{title}}</h2><!--rg-items--></section></template>
<template data-rg-group="poster"><header><!--rg-items--></header></template>
<template data-rg-widget="dial" data-rg-size="large" data-rg-label="Dial"><svg data-gauge="{{value}}"></svg><b data-cap="{{caption}}">{{{{value:.1}}}}</b></template>
<template data-rg-widget="bar" data-rg-size="small" data-rg-label="Bar" data-rg-colors="red blue"><i class="{{color}}" data-bar="{{value}}" data-max="{{max}}" id="b-{{slot}}"></i></template>
<template data-rg-widget="bar" data-rg-size="medium"><i class="wide" data-bar="{{value}}"></i></template>
<template data-rg-widget="clock" data-rg-size="large" data-rg-sources="clock" data-rg-group="poster"><time data-clock="{{time}}"></time></template>
<template data-rg-widget="fan" data-rg-size="small" data-rg-sources="fan"><b data-rotor="{{value}}"></b></template>
</body></html>"#;

    fn choice(kind: &str, size: &str) -> CssWidgetChoice {
        CssWidgetChoice {
            kind: kind.to_string(),
            size: size.to_string(),
            color: String::new(),
            pos: None,
            span: None,
        }
    }

    fn layout() -> CssLayoutConfig {
        CssLayoutConfig {
            custom: true,
            groups: vec![
                CssGroupLayout::default(),
                CssGroupLayout {
                    title: "CPU <1>".to_string(),
                    ..Default::default()
                },
            ],
            widgets: [
                ("group1_1".to_string(), choice("clock", "")),
                ("group2_1".to_string(), choice("dial", "")),
                ("group2_2".to_string(), choice("bar", "medium")),
                ("group2_3".to_string(), choice("bar", "")),
            ]
            .into(),
            item_counts: vec![1, 3],
        }
    }

    #[test]
    fn the_spec_lists_what_the_template_offers() {
        let spec = layout_spec(PAGE);
        assert!(spec.usable());
        assert_eq!(spec.own_indices, (0..=7).collect::<Vec<u32>>());
        let kinds = |source: &str| -> Vec<String> {
            spec.kinds_for(source).into_iter().map(|(k, _)| k).collect()
        };
        assert_eq!(kinds("cpu"), ["dial", "bar", HIDDEN]);
        assert_eq!(kinds("clock"), ["clock", HIDDEN]);
        // its own widget first, then the general ones
        assert_eq!(kinds("fan_speed"), ["fan", "dial", "bar", HIDDEN]);
        // a hidden sensor stays hidden
        assert_eq!(
            spec.coerce(Some(&choice(HIDDEN, "")), "cpu", "", 0).kind,
            HIDDEN
        );
        assert_eq!(spec.default_widget("fan_speed", "", 3).kind, "fan");
        assert_eq!(spec.default_widget("cpu", "Temperature", 2).kind, "dial");
        assert_eq!(spec.default_widget("cpu", "Usage", 2).kind, "bar");
        // a kind is drawn in the size asked for from the nearest prototype
        assert_eq!(spec.size_of(&choice("bar", "")), "small");
        assert_eq!(
            spec.widget(Some(&choice("bar", "large"))).unwrap().size,
            "medium"
        );
        assert!(!layout_spec("<p>{{0}}</p>").usable());
    }

    #[test]
    fn a_layout_that_is_off_leaves_the_template_alone() {
        assert!(expand_layout(PAGE, &CssLayoutConfig::default()).is_none());
        assert!(expand_layout("<p>{{0}}</p>", &layout()).is_none());
    }

    #[test]
    fn groups_and_widgets_are_built_from_the_layout() {
        let out = expand_layout(PAGE, &layout()).expect("expands");
        // the template's own groups are gone, what is outside them stays
        assert!(!out.html.contains(r#"data-num="1""#));
        assert!(out.html.contains(r#"data-uptime="73""#));
        // the clock chose the poster shell, the second group the first shell
        assert!(out.html.contains(r#"<header><div class="rg-items""#));
        assert!(out.html.contains("<h2>CPU &lt;1&gt;</h2>"));
        // a large dial, then a medium and a small bar in the row under it
        assert!(out.html.contains(
            "grid-template-columns:repeat(4,minmax(0,1fr));grid-template-rows:repeat(6,"
        ));
        assert!(out
            .html
            .contains(r#"data-rg-slot="group2_2" style="grid-column:1/span 2;grid-row:5/span 2""#));
        assert!(out
            .html
            .contains(r#"data-rg-slot="group2_3" style="grid-column:3/span 1;grid-row:5/span 1""#));
        // every token got a number of its own, bound to the widget's slot
        let find = |slot: &str, field: &str| {
            out.mappings
                .iter()
                .find(|m| m.slot_prefix == slot && m.field == field)
                .unwrap_or_else(|| panic!("{slot} {field}"))
        };
        let time = find("group1_1", "time").index;
        assert_eq!(time, FIRST_INDEX);
        assert!(out.html.contains(&format!(r#"data-clock="{time}""#)));
        let value = find("group2_1", "value").index;
        assert!(out.html.contains(&format!(r#"data-gauge="{value}""#)));
        // a formatted token is a separate placeholder, left as {{N}} text
        let shown = out
            .mappings
            .iter()
            .find(|m| m.slot_prefix == "group2_1" && m.format.as_deref() == Some("{:.1}"))
            .expect("formatted");
        assert!(out.html.contains(&format!("{{{{{}}}}}", shown.index)));
        // sizes: the medium bar and the small one are different prototypes
        assert!(out.html.contains(r#"<div class="rg-fit"><i class="wide""#));
        assert!(out.html.contains(r#"id="b-group2_3""#));
        let mut indices: Vec<u32> = out.mappings.iter().map(|m| m.index).collect();
        indices.dedup();
        assert_eq!(indices.len(), out.mappings.len(), "no number is used twice");
        // two groups: one column of two rows, or two columns
        assert!(out
            .html
            .contains(r#"name="rg-layouts" content="480x660 960x360""#));
        assert!(out
            .html
            .contains(r#"name="rg-design-size" content="480x660""#));
    }

    #[test]
    fn a_group_shows_six_widgets_at_most() {
        let mut l = layout();
        l.item_counts = vec![9];
        let out = expand_layout(PAGE, &l).expect("expands");
        assert_eq!(out.html.matches("rg-item ").count(), CSS_LAYOUT_MAX_ITEMS);
    }

    #[test]
    fn hidden_sensors_and_empty_groups_are_not_drawn() {
        let mut l = layout();
        l.widgets.insert("group1_1".to_string(), choice(HIDDEN, ""));
        l.widgets.insert("group2_2".to_string(), choice(HIDDEN, ""));
        let out = expand_layout(PAGE, &l).expect("expands");
        assert!(
            !out.html.contains("<header><div"),
            "the clock's group is gone"
        );
        assert_eq!(out.html.matches("rg-item ").count(), 2);
        assert!(out.mappings.iter().all(|m| m.slot_prefix != "group2_2"));
        // one group left: one arrangement
        assert!(out.html.contains(r#"name="rg-layouts" content="480x360""#));
        assert!(!out.html.contains(r#"data-rg-slot="group2_2""#));
    }

    #[test]
    fn colours_take_turns_unless_one_is_chosen() {
        let mut l = layout();
        l.item_counts = vec![0, 4];
        for i in 1..=4 {
            l.widgets
                .insert(format!("group2_{i}"), choice("bar", "small"));
        }
        l.widgets.get_mut("group2_3").unwrap().color = "blue".to_string();
        l.widgets.get_mut("group2_4").unwrap().color = "green".to_string(); // not offered
        let out = expand_layout(PAGE, &l).expect("expands");
        let built = &out.html[..out.html.find("<template").unwrap()];
        let classes: Vec<&str> = built
            .match_indices(r#"<i class=""#)
            .map(|(at, m)| &built[at + m.len()..at + m.len() + 4])
            .collect();
        assert_eq!(classes, ["red\"", "blue", "blue", "blue"]);
        let spec = layout_spec(PAGE);
        let mut picked = choice("bar", "small");
        picked.color = "blue".to_string();
        assert_eq!(spec.coerce(Some(&picked), "cpu", "", 0).color, "blue");
        // the medium bar has no colours: the choice does not carry over
        picked.size = "medium".to_string();
        assert_eq!(spec.coerce(Some(&picked), "cpu", "", 0).color, "");
        // where it was put is kept
        picked.pos = Some((2, 1));
        assert_eq!(spec.coerce(Some(&picked), "cpu", "", 0).pos, Some((2, 1)));
    }

    #[test]
    fn widgets_sit_where_they_are_put_and_the_rest_fill_in() {
        // four small ones down the left of a large one
        let sq = |n: u32| (n, n);
        let p = place(&[
            (sq(4), Some((1, 0))),
            (sq(1), Some((0, 0))),
            (sq(1), Some((0, 1))),
            (sq(1), Some((0, 2))),
            (sq(1), Some((0, 3))),
        ]);
        assert_eq!((p.cols, p.rows), (5, 4));
        assert_eq!(p.cells[0], (1, 0, 4, 4));
        assert_eq!(p.cells[4], (0, 3, 1, 1));
        // nothing placed: first free spot, four cells wide
        let p = place(&[
            (sq(4), None),
            (sq(2), None),
            (sq(1), None),
            (sq(1), None),
            (sq(2), None),
        ]);
        assert_eq!(
            p.cells,
            [
                (0, 0, 4, 4),
                (0, 4, 2, 2),
                (2, 4, 1, 1),
                (3, 4, 1, 1),
                (2, 5, 2, 2)
            ]
        );
        // the grid starts at the first cell used
        let p = place(&[(sq(2), Some((3, 3))), (sq(1), Some((5, 4)))]);
        assert_eq!(p.cells, [(0, 0, 2, 2), (2, 1, 1, 1)]);
        assert_eq!((p.cols, p.rows), (3, 2));
        // a cell that is taken is not used twice
        let p = place(&[(sq(2), Some((0, 0))), (sq(2), Some((1, 1)))]);
        assert_eq!(p.cells, [(0, 0, 2, 2), (2, 0, 2, 2)]);
    }

    #[test]
    fn a_widget_can_take_a_block_that_is_not_square() {
        // a dial over three bars, each a row of four cells: 4 x 7, no gaps
        let p = place(&[
            ((4, 4), None),
            ((4, 1), None),
            ((4, 1), None),
            ((4, 1), None),
        ]);
        assert_eq!(p.cells[3], (0, 6, 4, 1));
        assert_eq!((p.cols, p.rows), (4, 7));
        // a tall block beside a wide one does not collide with it
        let p = place(&[
            ((1, 3), Some((0, 0))),
            ((3, 1), Some((1, 0))),
            ((3, 1), Some((0, 1))),
        ]);
        assert_eq!(p.cells[2], (1, 1, 3, 1));

        let spec = layout_spec(include_str!("../../examples/bauhaus_panel.html"));
        let mut bar = choice("bar", "small");
        assert_eq!(spec.block_of(&bar), (1, 1));
        // shaped by the user: the drawing whose proportions suit the block
        bar.span = Some((4, 1));
        assert_eq!(spec.block_of(&bar), (4, 1));
        assert_eq!(spec.widget(Some(&bar)).unwrap().size, "medium");
        bar.span = Some((8, 1));
        assert_eq!(spec.widget(Some(&bar)).unwrap().size, "large");
        // kept through a coerce, within bounds
        bar.span = Some((40, 0));
        assert_eq!(
            spec.coerce(Some(&bar), "cpu", "", 0).span,
            Some((MAX_SPAN, 1))
        );

        let mut layout = CssLayoutConfig {
            custom: true,
            item_counts: vec![1],
            ..Default::default()
        };
        layout.widgets.insert(
            "group1_1".to_string(),
            CssWidgetChoice {
                span: Some((4, 1)),
                ..choice("bar", "")
            },
        );
        let out = expand_layout(PAGE, &layout).unwrap();
        assert!(out
            .html
            .contains(r#"style="grid-column:1/span 4;grid-row:1/span 1""#));

        // On the page the drawing follows the cells' real shape: under a
        // large dial the grid is 4 x 8 in a group wider than that, so a
        // 4 x 1 bar has room for the long drawing, not the stacked one.
        let bauhaus = include_str!("../../examples/bauhaus_panel.html");
        let mut layout = CssLayoutConfig {
            custom: true,
            item_counts: vec![5],
            ..Default::default()
        };
        layout
            .widgets
            .insert("group1_1".to_string(), choice("dial", "large"));
        for i in 2..=5 {
            let bar = CssWidgetChoice {
                span: Some((4, 1)),
                ..choice("bar", "")
            };
            layout.widgets.insert(format!("group1_{i}"), bar);
        }
        let out = expand_layout(bauhaus, &layout).unwrap();
        let built = &out.html[..out.html.find("<template").unwrap()];
        assert_eq!(built.matches(r#"<div class="stat" data-need"#).count(), 4);
        assert!(!built.contains("stat stacked"));
    }

    #[test]
    fn a_fresh_layout_puts_the_showpiece_beside_its_bars() {
        let spec = layout_spec(PAGE);
        // the first of a group and a temperature: the showpiece at 2 x 2
        let first = spec.default_widget("cpu", "Usage", 0);
        assert_eq!(
            (first.kind.as_str(), spec.block_of(&first)),
            ("dial", (2, 2))
        );
        let hot = spec.default_widget("gpu", "Temperature", 3);
        assert_eq!(hot.kind, "dial");
        // the rest: the second kind as a 2 x 1 block, so the bars stack
        // beside the showpiece
        let rest = spec.default_widget("cpu", "Frequency", 1);
        assert_eq!((rest.kind.as_str(), spec.block_of(&rest)), ("bar", (2, 1)));
        let p = place(&[
            ((2, 2), None),
            ((2, 1), None),
            ((2, 1), None),
            ((2, 1), None),
        ]);
        assert_eq!(
            p.cells,
            [(0, 0, 2, 2), (2, 0, 2, 1), (2, 1, 2, 1), (0, 2, 2, 1)]
        );
        // a widget made for the source keeps its own size
        let fan = spec.default_widget("fan_speed", "", 0);
        assert_eq!((fan.kind.as_str(), fan.span), ("fan", None));

        // `data-rg-default-for`: first choice for those sources, on offer
        // for every other one
        let page = PAGE.replace(
            r#"<template data-rg-widget="fan""#,
            r#"<template data-rg-widget="vessel" data-rg-size="medium" data-rg-default-for="memory disk"><b data-fill="{{value}}"></b></template>
<template data-rg-widget="fan""#,
        );
        let spec = layout_spec(&page);
        let memory = spec.default_widget("memory", "Used", 2);
        assert_eq!(
            (memory.kind.as_str(), memory.size.as_str()),
            ("vessel", "medium")
        );
        assert_eq!(spec.default_widget("cpu", "Usage", 1).kind, "bar");
        assert!(spec.kinds_for("cpu").iter().any(|(k, _)| k == "vessel"));
    }

    #[test]
    fn group_shells_know_their_place() {
        let page = PAGE.replace(
            r#"<section><h2>{{title}}</h2>"#,
            r#"<section class="g{{index}} {{parity}}"><h2>{{title}}</h2>"#,
        );
        let mut layout = CssLayoutConfig {
            custom: true,
            item_counts: vec![1, 1, 1],
            ..Default::default()
        };
        // the middle group draws nothing, so the third is the second drawn
        layout
            .widgets
            .insert("group2_1".to_string(), choice(HIDDEN, ""));
        for slot in ["group1_1", "group3_1"] {
            layout.widgets.insert(slot.to_string(), choice("dial", ""));
        }
        let out = expand_layout(&page, &layout).unwrap();
        assert!(out.html.contains(r#"<section class="g1 odd">"#));
        assert!(out.html.contains(r#"<section class="g2 even">"#));
    }

    #[test]
    fn a_widget_follows_its_source() {
        let spec = layout_spec(PAGE);
        // a dial on a slot that became a clock is replaced
        assert_eq!(
            spec.coerce(Some(&choice("dial", "")), "clock", "", 0).kind,
            "clock"
        );
        // a size the kind does not come in is dropped, the kind kept
        assert_eq!(
            spec.coerce(Some(&choice("bar", "huge")), "cpu", "", 1),
            choice("bar", "")
        );
        assert_eq!(
            spec.coerce(Some(&choice("bar", "small")), "cpu", "", 1),
            choice("bar", "small")
        );
    }

    /// The Bauhaus example, set up by Scan Placeholders and switched to a
    /// custom layout: every slot gets a widget its source is offered, the
    /// clock its poster, and no token is left unresolved.
    #[test]
    fn the_bauhaus_example_builds_a_layout_from_a_scanned_panel() {
        use crate::ui::config_tree::util::{add_wanted_slots, combo_slot_kinds};
        use crate::ui::css_template_display::{
            detect_placeholders, extract_placeholder_defaults, wanted_slots,
        };
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/bauhaus_panel.html");
        let html = std::fs::read_to_string(path).unwrap();
        let spec = layout_spec(&html);
        assert!(spec.usable());
        assert!(spec.kinds().len() >= 5);

        let mut source = crate::sources::ComboSourceConfig::default();
        let defaults = extract_placeholder_defaults(&html);
        add_wanted_slots(
            &mut source,
            &wanted_slots(&detect_placeholders(&html), &defaults),
        );
        let slots = combo_slot_kinds(&source);
        let mut layout = CssLayoutConfig {
            custom: true,
            ..Default::default()
        };
        let mappings = crate::ui::css_template_display::auto_map(
            &detect_placeholders(&html),
            &defaults,
            &HashMap::new(),
            &slots,
        );
        coerce_layout(&mut layout, &spec, &slots, &mappings);
        if let Ok(path) = std::env::var("LAYOUT_JSON") {
            std::fs::write(path, serde_json::to_string_pretty(&layout).unwrap()).unwrap();
        }
        // uptime is only in the footer: bound, but not a widget
        let uptime = slots
            .iter()
            .find(|s| s.source == "uptime")
            .expect("uptime slot");
        assert_eq!(layout.widgets[&uptime.prefix].kind, HIDDEN);
        assert_eq!(layout.item_counts.len(), source.groups.len());
        for slot in &slots {
            let kind = &layout.widgets[&slot.prefix].kind;
            assert_eq!(*kind == "clock", slot.source == "clock", "{}", slot.prefix);
        }

        let out = expand_layout(&html, &layout).expect("expands");
        let built = &out.html[..out.html.find("<template").unwrap()];
        assert!(!TOKEN.is_match(built), "a token was left in the page");
        assert!(built.contains(r#"<header class="poster">"#));
        // the template's own widgets are gone, the footer's uptime is not
        assert!(!built.contains(r#"data-gauge="5""#));
        assert!(built.contains(r#"data-uptime="73""#));
        let shown = layout.widgets.values().filter(|w| w.kind != HIDDEN).count();
        assert_eq!(built.matches(r#"class="rg-item "#).count(), shown);
        assert_eq!(
            built.matches("data-section").count() + built.matches(r#"class="poster""#).count(),
            source.groups.len() - 1,
            "the uptime group is not drawn"
        );
        // and with the layout off nothing changes at all
        layout.custom = false;
        assert!(expand_layout(&html, &layout).is_none());
    }

    /// Every example panel that declares prototypes: a freshly scanned
    /// panel builds into a page with nothing left over.
    #[test]
    fn example_panels_with_prototypes_build_a_scanned_layout() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
        let mut checked = 0;
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if !name.ends_with("_panel.html") {
                continue;
            }
            let html = std::fs::read_to_string(&path).unwrap();
            let spec = layout_spec(&html);
            if spec.widgets.is_empty() && spec.groups.is_empty() && !spec.has_region {
                continue;
            }
            checked += 1;
            assert!(spec.usable(), "{name}: prototypes, groups and a region");
            assert!(spec.group_size.is_some(), "{name}: rg-group-size");
            assert!(
                !spec.own_indices.is_empty(),
                "{name}: the region lists no placeholders"
            );
            for w in &spec.widgets {
                // (a widget for one source may size itself by its content)
                assert!(
                    !w.sources.is_empty() || w.natural.is_some_and(|(w, h)| w > 0 && h > 0),
                    "{name}: {} ({}) states no data-rg-width / data-rg-height",
                    w.kind,
                    w.size
                );
            }
            // a prototype is a whole piece of markup: an element left open
            // swallows the widgets after it into its own box
            let bodies = spec
                .widgets
                .iter()
                .map(|w| (format!("{} ({})", w.kind, w.size), &w.html))
                .chain(spec.groups.iter().map(|g| (g.id.clone(), &g.html)));
            for (what, body) in bodies {
                for tag in ["div", "span", "section", "header", "svg", "g"] {
                    let opened = Regex::new(&format!(r"<{tag}[\s>]")).unwrap();
                    assert_eq!(
                        opened.find_iter(body).count(),
                        body.matches(&format!("</{tag}>")).count(),
                        "{name}: {what} leaves a <{tag}> open (or closes one too many)"
                    );
                }
            }
            // `{{{{value}}}}` is text in braces ("{1012}"): right for a text
            // placeholder, never for an attribute the script reads as an id
            let braced = Regex::new(r#"="\{\{\{\{[^"]*\}\}\}\}""#).unwrap();
            for w in &spec.widgets {
                assert!(
                    !braced.is_match(&w.html),
                    "{name}: {} ({}) has an attribute in double braces",
                    w.kind,
                    w.size
                );
            }
            // something for any sensor, and for a clock
            assert!(
                spec.widgets.iter().any(|w| w.sources.is_empty()),
                "{name}: no general widget"
            );

            let scanned = scan_template(&html);
            let mut layout = scanned.layout;
            assert!(layout.custom, "{name}");
            let has_clock_widget = spec
                .widgets
                .iter()
                .any(|w| LayoutSpec::made_for(w, "clock"));
            for slot in &scanned.slots {
                let kind = &layout.widgets[&slot.prefix].kind;
                if has_clock_widget && slot.source == "clock" {
                    assert!(
                        spec.widgets
                            .iter()
                            .any(|w| w.kind == *kind && LayoutSpec::made_for(w, "clock")),
                        "{name}: {} is a clock shown as {kind}",
                        slot.prefix
                    );
                }
            }
            let out = expand_layout(&html, &layout).unwrap_or_else(|| panic!("{name}: expands"));
            let built = &out.html[..out.html.find("<template").unwrap()];
            assert!(
                !TOKEN.is_match(built),
                "{name}: a token was left in the page"
            );
            assert!(
                !GROUPS_REGION.is_match(&out.html),
                "{name}: the region is still there"
            );
            let shown = layout.widgets.values().filter(|w| w.kind != HIDDEN).count();
            assert!(shown > 0, "{name}: nothing is shown");
            assert_eq!(built.matches(r#"class="rg-item "#).count(), shown, "{name}");
            // the generated numbers are clear of the template's own
            let own_max = crate::ui::css_template_display::detect_placeholders(&html)
                .into_iter()
                .max()
                .unwrap_or(0);
            assert!(own_max < FIRST_INDEX, "{name}: placeholder {own_max}");
            layout.custom = false;
            assert!(expand_layout(&html, &layout).is_none(), "{name}");
        }
        assert!(checked >= 1, "Bauhaus at least");
        if let Ok(want) = std::env::var("EXPECT_PROTOTYPE_PANELS") {
            assert_eq!(checked.to_string(), want);
        }
    }

    #[test]
    fn layouts_follow_the_group_count() {
        assert_eq!(layouts_for(1, (480, 300, 60)), "480x360");
        // four columns would need the same two rows as three: left out
        assert_eq!(
            layouts_for(5, (480, 300, 0)),
            "480x1500 960x900 1440x600 2400x300"
        );
    }
}
