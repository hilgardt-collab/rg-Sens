//! Tree nodes of the properties dialog.
//!
//! One enum describes every selectable thing. Rows carry a `NodeKind` in a
//! `glib::BoxedAnyObject`; pages in the editor stack are keyed by
//! [`NodeKind::id`]. `Section` is reserved for Phase 3, when an editor can
//! expose its own sub-sections as child nodes.

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
    /// One detached page of a themed combo editor (Frame, Headers, …).
    DisplaySection(String),
    /// Combo panels: group count, update interval, content-wide settings.
    Content,
    /// One content group (0-based index).
    ContentGroup(usize),
    /// One content item, by slot key (`"group{N}_{M}"`).
    ContentItem(String),
    Background,
    Appearance,
    /// Reserved: an editor-provided sub-section (Phase 3).
    #[allow(dead_code)]
    Section(Vec<String>),
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
            NodeKind::DisplaySection(n) => format!("section/{}", n),
            NodeKind::Content => "content".into(),
            NodeKind::ContentGroup(g) => format!("group/{}", g),
            NodeKind::ContentItem(s) => format!("item/{}", s),
            NodeKind::Background => "background".into(),
            NodeKind::Appearance => "appearance".into(),
            NodeKind::Section(path) => format!("sec/{}", path.join("/")),
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
