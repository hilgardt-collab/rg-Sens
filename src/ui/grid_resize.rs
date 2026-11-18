//! The arithmetic of resizing a panel by dragging its edges: which edge a
//! point is on, what the drag makes of the panel's cells, and whether the
//! result fits. No GTK here; `grid_layout.rs` wires it to one gesture on the
//! grid container, which serves every panel however its widgets were built.

use crate::core::PanelGeometry;
use std::collections::HashSet;

/// How far into a panel an edge can be grabbed (px).
const HANDLE_PX: f64 = 8.0;

/// The edges of a panel a drag moves. A corner is two of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Edges {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

impl Edges {
    fn any(self) -> bool {
        self.left || self.right || self.top || self.bottom
    }

    /// The CSS cursor that shows what a drag from here does.
    pub(crate) fn cursor(self) -> &'static str {
        let horizontal = self.left || self.right;
        let vertical = self.top || self.bottom;
        if (self.left && self.top) || (self.right && self.bottom) {
            "nwse-resize"
        } else if horizontal && vertical {
            "nesw-resize"
        } else if horizontal {
            "ew-resize"
        } else {
            "ns-resize"
        }
    }
}

/// The edges of the rectangle `(x, y, width, height)` that the point is on,
/// `None` when it is outside the rectangle or in its middle. The band is
/// inside the rectangle (panels are only a few pixels apart) and never more
/// than a quarter of a side, so the middle of a small panel still moves it.
pub(crate) fn hit_edges(px: f64, py: f64, rect: (f64, f64, f64, f64)) -> Option<Edges> {
    let (x, y, w, h) = rect;
    if w <= 0.0 || h <= 0.0 || px < x || py < y || px > x + w || py > y + h {
        return None;
    }
    let band_x = HANDLE_PX.min(w / 4.0);
    let band_y = HANDLE_PX.min(h / 4.0);
    let edges = Edges {
        left: px - x < band_x,
        right: x + w - px < band_x,
        top: py - y < band_y,
        bottom: y + h - py < band_y,
    };
    edges.any().then_some(edges)
}

/// The panel's cells after its `edges` were dragged by `(offset_x, offset_y)`
/// pixels on a grid whose cells repeat every `(pitch_x, pitch_y)` pixels. The
/// opposite edges stay where they are, the panel keeps at least one cell each
/// way and never leaves the grid's top or left.
pub(crate) fn resized(
    orig: PanelGeometry,
    edges: Edges,
    offset: (f64, f64),
    pitch: (f64, f64),
) -> PanelGeometry {
    let cells = |offset: f64, pitch: f64| -> i64 {
        if pitch > 0.0 && offset.is_finite() {
            (offset / pitch).round() as i64
        } else {
            0
        }
    };
    // one axis: (start, length) after moving the near or the far edge
    let axis = |start: u32, len: u32, near: bool, far: bool, delta: i64| -> (u32, u32) {
        let (start, len) = (start as i64, len.max(1) as i64);
        let end = start + len;
        if near {
            let new_start = (start + delta).clamp(0, end - 1);
            (new_start as u32, (end - new_start) as u32)
        } else if far {
            let new_len = (len + delta).clamp(1, u32::MAX as i64 - start);
            (start as u32, new_len as u32)
        } else {
            (start as u32, len as u32)
        }
    };
    let (x, width) = axis(
        orig.x,
        orig.width,
        edges.left,
        edges.right,
        cells(offset.0, pitch.0),
    );
    let (y, height) = axis(
        orig.y,
        orig.height,
        edges.top,
        edges.bottom,
        cells(offset.1, pitch.1),
    );
    PanelGeometry {
        x,
        y,
        width,
        height,
    }
}

/// Whether `cell` is one of the panel's.
pub(crate) fn contains(geometry: &PanelGeometry, cell: (u32, u32)) -> bool {
    cell.0 >= geometry.x
        && cell.0 - geometry.x < geometry.width
        && cell.1 >= geometry.y
        && cell.1 - geometry.y < geometry.height
}

/// Whether a panel that now has the cells of `own` would overlap another
/// panel with the cells of `new`. Its own cells are in `occupied` and do not
/// count.
pub(crate) fn collides(
    occupied: &HashSet<(u32, u32)>,
    own: &PanelGeometry,
    new: &PanelGeometry,
) -> bool {
    (0..new.width).any(|dx| {
        (0..new.height).any(|dy| {
            let cell = (new.x + dx, new.y + dy);
            !contains(own, cell) && occupied.contains(&cell)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geom(x: u32, y: u32, width: u32, height: u32) -> PanelGeometry {
        PanelGeometry {
            x,
            y,
            width,
            height,
        }
    }

    fn tuple(g: PanelGeometry) -> (u32, u32, u32, u32) {
        (g.x, g.y, g.width, g.height)
    }

    const RIGHT: Edges = Edges {
        left: false,
        right: true,
        top: false,
        bottom: false,
    };
    const TOP_LEFT: Edges = Edges {
        left: true,
        right: false,
        top: true,
        bottom: false,
    };

    #[test]
    fn edges_are_found_inside_the_panel_only() {
        let rect = (100.0, 50.0, 200.0, 100.0);
        assert_eq!(hit_edges(150.0, 100.0, rect), None, "the middle moves");
        assert_eq!(hit_edges(99.0, 100.0, rect), None, "outside");
        assert_eq!(hit_edges(297.0, 100.0, rect), Some(RIGHT));
        assert_eq!(hit_edges(102.0, 52.0, rect), Some(TOP_LEFT));
        let bottom = hit_edges(200.0, 148.0, rect).unwrap();
        assert!(bottom.bottom && !bottom.left && !bottom.right && !bottom.top);
    }

    #[test]
    fn a_one_cell_panel_keeps_a_middle_to_move_it_by() {
        let rect = (0.0, 0.0, 16.0, 16.0);
        assert_eq!(hit_edges(8.0, 8.0, rect), None);
        assert_eq!(hit_edges(14.0, 8.0, rect), Some(RIGHT));
    }

    #[test]
    fn cursors_name_the_direction() {
        assert_eq!(RIGHT.cursor(), "ew-resize");
        assert_eq!(TOP_LEFT.cursor(), "nwse-resize");
        let top_right = Edges {
            right: true,
            top: true,
            ..Edges::default()
        };
        assert_eq!(top_right.cursor(), "nesw-resize");
        let bottom = Edges {
            bottom: true,
            ..Edges::default()
        };
        assert_eq!(bottom.cursor(), "ns-resize");
    }

    #[test]
    fn a_far_edge_changes_the_size_and_snaps_to_cells() {
        let orig = geom(2, 3, 4, 5);
        let pitch = (19.0, 19.0);
        assert_eq!(
            tuple(resized(orig, RIGHT, (40.0, 300.0), pitch)),
            (2, 3, 6, 5)
        );
        assert_eq!(tuple(resized(orig, RIGHT, (9.0, 0.0), pitch)), (2, 3, 4, 5));
        // never narrower than one cell
        assert_eq!(
            tuple(resized(orig, RIGHT, (-500.0, 0.0), pitch)),
            (2, 3, 1, 5)
        );
    }

    #[test]
    fn a_near_edge_moves_the_origin_and_keeps_the_far_edge() {
        let orig = geom(2, 3, 4, 5);
        let pitch = (19.0, 19.0);
        assert_eq!(
            tuple(resized(orig, TOP_LEFT, (-19.0, 38.0), pitch)),
            (1, 5, 5, 3)
        );
        // stops at the grid's edge and at one cell
        assert_eq!(
            tuple(resized(orig, TOP_LEFT, (-500.0, 500.0), pitch)),
            (0, 7, 6, 1)
        );
    }

    #[test]
    fn its_own_cells_do_not_collide() {
        let own = geom(0, 0, 2, 2);
        let mut occupied: HashSet<(u32, u32)> =
            [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().collect();
        occupied.insert((3, 0)); // another panel
        assert!(!collides(&occupied, &own, &geom(0, 0, 3, 2)));
        assert!(!collides(&occupied, &own, &geom(1, 1, 1, 1)));
        assert!(collides(&occupied, &own, &geom(0, 0, 4, 2)));
    }
}
