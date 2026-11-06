//! Tree nodes of the properties dialog.
//!
//! One enum describes every selectable thing. Rows carry a `NodeKind` in a
//! `glib::BoxedAnyObject`; pages in the editor stack are keyed by
//! [`NodeKind::id`]. `Section` is a page the display editor provides,
//! addressed by path (a section, or a list item beneath one); `ItemSection`
//! is the same for a combo item's display editor, and `ItemSource` the
//! item's source settings.

use gtk4::glib;
use gtk4::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
    /// The panel itself.
    Root,
    /// Size, content transform, z-index, collision.
    Geometry,
    /// Theme reference chooser (+ inline editor).
    Theme,
    /// Standalone panels: source type + its editor.
    Source,
    /// Displayer type (+ the editor for standalone displayers).
    Display,
    /// Combo panels: group count, update interval, content-wide settings.
    Content,
    /// One content group (0-based index).
    ContentGroup(usize),
    /// One content item, by slot key (`"group{N}_{M}"`).
    ContentItem(String),
    Background,
    Appearance,
    /// A page of the display editor by path: `["Face"]` is a section,
    /// `["Text overlay", "lines", "0"]` a list item beneath one.
    Section(Vec<String>),
    /// The source settings of one content item, by slot key.
    ItemSource(String),
    /// A page of one content item's display editor: the slot key and the
    /// path, shaped as in `Section` (relative to the item).
    ItemSection(String, Vec<String>),
}

impl NodeKind {
    /// Stable id, used as the editor stack page name.
    pub fn id(&self) -> String {
        match self {
            NodeKind::Root => "root".into(),
            NodeKind::Geometry => "geometry".into(),
            NodeKind::Theme => "theme".into(),
            NodeKind::Source => "source".into(),
            NodeKind::Display => "display".into(),
            NodeKind::Content => "content".into(),
            NodeKind::ContentGroup(g) => format!("group/{}", g),
            NodeKind::ContentItem(s) => format!("item/{}", s),
            NodeKind::Background => "background".into(),
            NodeKind::Appearance => "appearance".into(),
            NodeKind::Section(path) => format!("sec/{}", path.join("/")),
            NodeKind::ItemSource(slot) => format!("itemsrc/{}", slot),
            NodeKind::ItemSection(slot, path) => format!("item/{}/{}", slot, path.join("/")),
        }
    }

    pub fn boxed(self) -> glib::BoxedAnyObject {
        glib::BoxedAnyObject::new(self)
    }

    pub fn from_object(obj: &glib::Object) -> Option<NodeKind> {
        obj.downcast_ref::<glib::BoxedAnyObject>()
            .map(|b| b.borrow::<NodeKind>().clone())
    }

    /// Parse a `"group{N}_{M}"` slot key into 0-based `(group, item)`.
    pub fn parse_slot(slot: &str) -> Option<(usize, usize)> {
        let rest = slot.strip_prefix("group")?;
        let (g, m) = rest.split_once('_')?;
        let g: usize = g.parse().ok()?;
        let m: usize = m.parse().ok()?;
        if g == 0 || m == 0 {
            return None;
        }
        Some((g - 1, m - 1))
    }
}
