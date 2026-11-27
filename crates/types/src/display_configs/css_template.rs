//! CSS Template display configuration types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for a placeholder mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceholderMapping {
    /// Placeholder index (0, 1, 2...)
    pub index: u32,
    /// Slot prefix from combo source (e.g., "group1_1")
    pub slot_prefix: String,
    /// Field to use (e.g., "value", "caption", "unit", "percent")
    pub field: String,
    /// Optional format string (e.g., "{:.1}%")
    #[serde(default)]
    pub format: Option<String>,
}

impl Default for PlaceholderMapping {
    fn default() -> Self {
        Self {
            index: 0,
            slot_prefix: String::new(),
            field: "value".to_string(),
            format: None,
        }
    }
}

/// Default configuration for a placeholder (from template)
///
/// This defines what source type and field a placeholder expects,
/// allowing auto-configuration when the template is loaded.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaceholderDefault {
    /// Human-readable description/hint for this placeholder
    #[serde(default)]
    pub hint: String,
    /// Source type ID (e.g., "cpu", "gpu", "memory", "clock", "disk")
    #[serde(default)]
    pub source: String,
    /// What the slot must measure, as the source's own field name
    /// (`"temperature"`, `"memory_used"`, `"download_speed"`); compared
    /// ignoring case and underscores. Empty accepts any slot of the source.
    #[serde(default)]
    pub metric: String,
    /// Which matching slot: 0 takes the next one no other placeholder has
    /// taken, N the Nth (so several placeholders can read one slot).
    #[serde(default)]
    pub instance: u32,
    /// Field to use from the source (e.g., "value", "caption", "unit", "time")
    #[serde(default = "default_field")]
    pub field: String,
    /// Optional format string (e.g., "{:.1}%")
    #[serde(default)]
    pub format: Option<String>,
}

fn default_field() -> String {
    "value".to_string()
}

/// Configuration for the CSS Template displayer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssTemplateDisplayConfig {
    /// Path to the HTML template file
    #[serde(default)]
    pub html_path: PathBuf,
    /// Optional path to external CSS file
    #[serde(default)]
    pub css_path: Option<PathBuf>,
    /// Mappings from placeholder indices to data sources
    #[serde(default)]
    pub mappings: Vec<PlaceholderMapping>,
    /// Enable hot-reload when template files change
    #[serde(default = "default_hot_reload")]
    pub hot_reload: bool,
    /// Background color for the WebView (RGBA)
    #[serde(default = "default_background_color")]
    pub background_color: [f64; 4],
    /// Enable CSS animations
    #[serde(default = "default_animation_enabled")]
    pub animation_enabled: bool,
    /// Animation speed multiplier
    #[serde(default = "default_animation_speed")]
    pub animation_speed: f64,
    /// Embedded HTML content (used when no file is specified)
    #[serde(default)]
    pub embedded_html: Option<String>,
    /// Embedded CSS content (used when no file is specified)
    #[serde(default)]
    pub embedded_css: Option<String>,
    /// Values chosen for the options the template declares
    /// (`rg-template-options`), by option id. An option at its default has
    /// no entry; entries for options the current template does not declare
    /// are kept but not applied.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub options: std::collections::BTreeMap<String, String>,
    /// The user's own arrangement of groups and widgets, for a template that
    /// declares widget prototypes. Off (and not written) by default: the
    /// template's own layout is shown.
    #[serde(default, skip_serializing_if = "CssLayoutConfig::is_default")]
    pub layout: CssLayoutConfig,
}

/// How one group of a custom layout is presented.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CssGroupLayout {
    /// The group's heading; empty shows `Group N`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    /// Which of the template's group prototypes draws it; empty follows the
    /// group's first widget.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub style: String,
}

/// The widget one slot is shown with.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CssWidgetChoice {
    /// The prototype's kind (`dial`, `bar`).
    #[serde(default)]
    pub kind: String,
    /// `large`, `medium` or `small`; empty takes the kind's first size.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub size: String,
    /// One of the colours the widget offers; empty lets the template's
    /// colours take turns through the group.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// Where it sits in its group, as `(column, row)` of the group's grid of
    /// square cells (a small widget takes 1x1, a medium 2x2, a large 4x4).
    /// `None` is placed in the first free spot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<(u32, u32)>,
    /// The block of cells it takes, as `(columns, rows)`, when that is not
    /// the square its size stands for (a bar at 4x1). `None` takes the
    /// size's square.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<(u32, u32)>,
}

/// A custom-layout group shows at most this many widgets.
pub const CSS_LAYOUT_MAX_ITEMS: usize = 6;

/// A custom layout: which widget prototype of the template shows each slot
/// of the panel's combination source.
///
/// The source's `groups[]` owns the structure. `item_counts` is a cache of
/// it, written by `DisplayerConfig::sync_group_structure`; `groups` is kept
/// parallel to it and `widgets` is keyed by slot name (`group1_2`), so both
/// follow a reorder.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CssLayoutConfig {
    /// Build the page's groups from this layout instead of the template's own.
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub groups: Vec<CssGroupLayout>,
    /// Slot name -> widget. A slot without an entry (or naming a kind the
    /// template lacks) gets the template's first widget.
    #[serde(default)]
    pub widgets: std::collections::BTreeMap<String, CssWidgetChoice>,
    #[serde(default)]
    pub item_counts: Vec<u32>,
}

impl CssLayoutConfig {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Adopt the source's structure: one entry per group, widgets only for
    /// slots that exist.
    pub fn sync(&mut self, item_counts: Vec<u32>) {
        if !self.custom && self.is_default() {
            return; // never touched: stays unwritten
        }
        self.groups
            .resize(item_counts.len(), CssGroupLayout::default());
        self.widgets.retain(|slot, _| {
            parse_slot(slot)
                .is_some_and(|(g, i)| item_counts.get(g).is_some_and(|n| i < *n as usize))
        });
        self.item_counts = item_counts;
    }

    pub fn swap_groups(&mut self, a: usize, b: usize) {
        use crate::combo::reorder::remap_group_swap;
        if a < self.groups.len() && b < self.groups.len() {
            self.groups.swap(a, b);
        }
        if a < self.item_counts.len() && b < self.item_counts.len() {
            self.item_counts.swap(a, b);
        }
        self.widgets = std::mem::take(&mut self.widgets)
            .into_iter()
            .map(|(slot, w)| (remap_group_swap(&slot, a + 1, b + 1), w))
            .collect();
    }

    pub fn swap_items(&mut self, group: usize, i: usize, j: usize) {
        use crate::combo::reorder::remap_item_swap;
        self.widgets = std::mem::take(&mut self.widgets)
            .into_iter()
            .map(|(slot, w)| (remap_item_swap(&slot, group + 1, i + 1, j + 1), w))
            .collect();
    }
}

fn default_hot_reload() -> bool {
    true
}

fn default_background_color() -> [f64; 4] {
    [0.0, 0.0, 0.0, 0.0] // Transparent
}

fn default_animation_enabled() -> bool {
    true
}

fn default_animation_speed() -> f64 {
    1.0
}

impl Default for CssTemplateDisplayConfig {
    fn default() -> Self {
        Self {
            html_path: PathBuf::new(),
            css_path: None,
            mappings: Vec::new(),
            hot_reload: default_hot_reload(),
            background_color: default_background_color(),
            animation_enabled: default_animation_enabled(),
            animation_speed: default_animation_speed(),
            embedded_html: None,
            embedded_css: None,
            options: std::collections::BTreeMap::new(),
            layout: CssLayoutConfig::default(),
        }
    }
}

/// `group2_3` as zero-based `(1, 2)`.
pub fn parse_slot(slot: &str) -> Option<(usize, usize)> {
    let (g, m) = slot.strip_prefix("group")?.split_once('_')?;
    let (g, m): (usize, usize) = (g.parse().ok()?, m.parse().ok()?);
    (g > 0 && m > 0).then(|| (g - 1, m - 1))
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    fn layout() -> CssLayoutConfig {
        let widget = |kind: &str| CssWidgetChoice {
            kind: kind.to_string(),
            ..Default::default()
        };
        CssLayoutConfig {
            custom: true,
            groups: vec![
                CssGroupLayout {
                    title: "one".into(),
                    ..Default::default()
                },
                CssGroupLayout {
                    title: "two".into(),
                    ..Default::default()
                },
            ],
            widgets: [
                ("group1_1".to_string(), widget("clock")),
                ("group2_1".to_string(), widget("dial")),
                ("group2_2".to_string(), widget("bar")),
            ]
            .into(),
            item_counts: vec![1, 2],
        }
    }

    #[test]
    fn an_untouched_layout_is_not_written() {
        let mut config = CssTemplateDisplayConfig::default();
        config.layout.sync(vec![2, 3]);
        let json = serde_json::to_value(&config).unwrap();
        assert!(json.get("layout").is_none());
    }

    #[test]
    fn the_layout_follows_the_source_structure_and_reorders() {
        let mut l = layout();
        l.swap_groups(0, 1);
        assert_eq!(l.groups[0].title, "two");
        assert_eq!(l.item_counts, vec![2, 1]);
        assert_eq!(l.widgets["group1_2"].kind, "bar");
        assert_eq!(l.widgets["group2_1"].kind, "clock");
        l.swap_items(0, 0, 1);
        assert_eq!(l.widgets["group1_1"].kind, "bar");
        // the first group loses an item, a third group appears
        l.sync(vec![1, 1, 1]);
        assert_eq!(l.groups.len(), 3);
        assert!(!l.widgets.contains_key("group1_2"));
        assert!(l.widgets.contains_key("group2_1"));
    }
}
