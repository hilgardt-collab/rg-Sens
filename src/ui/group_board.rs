//! Where a combo panel's groups sit, as blocks on a grid of cells: the
//! arithmetic behind the Content page's group-size board.
//!
//! A group's size weight is a whole number of cells (`span_of`, quarters of
//! a weight), so the board can be edited the way a group's widgets are: by
//! dragging a block's edge across the cells. A themed frame stacks its
//! groups one way (`Flow::Down` / `Flow::Across`); a CSS custom layout puts
//! them in the page's columns, each spanning rows by its weight, placed as
//! CSS grid auto-placement places them (`Flow::Columns`).

/// Cells per unit of weight: a weight of 1 is four cells, the least a group
/// can have is one (0.25).
pub const STEPS: f64 = 4.0;
/// The most cells a group can span (a weight of 8).
pub const MAX_SPAN: u32 = 32;

/// How the groups are laid out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Flow {
    /// One column, top to bottom.
    Down,
    /// One row, left to right.
    Across,
    /// A grid of this many columns, filled row by row.
    Columns(u32),
}

/// One group's block, in cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    pub col: u32,
    pub row: u32,
    pub wide: u32,
    pub high: u32,
}

/// The groups' blocks on a grid of `cols` x `rows` cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Board {
    pub cols: u32,
    pub rows: u32,
    pub blocks: Vec<Block>,
}

/// A weight as cells.
pub fn span_of(weight: f64) -> u32 {
    let w = if weight.is_finite() && weight > 0.0 {
        weight
    } else {
        1.0
    };
    ((w * STEPS).round() as u32).clamp(1, MAX_SPAN)
}

/// Cells as a weight.
pub fn weight_of(span: u32) -> f64 {
    span.clamp(1, MAX_SPAN) as f64 / STEPS
}

/// Lay out groups of `spans` cells each.
pub fn arrange(spans: &[u32], flow: Flow) -> Board {
    let spans: Vec<u32> = spans.iter().map(|s| (*s).clamp(1, MAX_SPAN)).collect();
    match flow {
        Flow::Down | Flow::Across => {
            let mut at = 0;
            let blocks: Vec<Block> = spans
                .iter()
                .map(|&n| {
                    let b = if flow == Flow::Down {
                        Block {
                            col: 0,
                            row: at,
                            wide: 1,
                            high: n,
                        }
                    } else {
                        Block {
                            col: at,
                            row: 0,
                            wide: n,
                            high: 1,
                        }
                    };
                    at += n;
                    b
                })
                .collect();
            let total = at.max(1);
            if flow == Flow::Down {
                Board {
                    cols: 1,
                    rows: total,
                    blocks,
                }
            } else {
                Board {
                    cols: total,
                    rows: 1,
                    blocks,
                }
            }
        }
        Flow::Columns(cols) => {
            // CSS grid auto-placement, sparse: a cursor that only moves
            // forward; each group takes the first column at or after it
            // whose cells are free for its whole height.
            let cols = cols.max(1);
            let mut taken: Vec<Vec<bool>> = Vec::new();
            let free = |taken: &Vec<Vec<bool>>, r: u32, c: u32, n: u32| {
                (r..r + n).all(|r| taken.get(r as usize).is_none_or(|row| !row[c as usize]))
            };
            let (mut row, mut col) = (0u32, 0u32);
            let mut blocks = Vec::with_capacity(spans.len());
            for &n in &spans {
                loop {
                    if col >= cols {
                        col = 0;
                        row += 1;
                    }
                    if free(&taken, row, col, n) {
                        break;
                    }
                    col += 1;
                }
                for r in row..row + n {
                    while taken.len() <= r as usize {
                        taken.push(vec![false; cols as usize]);
                    }
                    taken[r as usize][col as usize] = true;
                }
                blocks.push(Block {
                    col,
                    row,
                    wide: 1,
                    high: n,
                });
                col += 1;
            }
            Board {
                cols,
                rows: (taken.len() as u32).max(1),
                blocks,
            }
        }
    }
}

/// The number of columns a CSS template's page shows: `forced` when the
/// template's columns option asks for a count, else the arrangement of
/// `layouts` (each the size the design needs with 1, 2, ... columns) that can
/// be shown largest in a panel of `panel` pixels, as the page's bridge script
/// chooses it.
pub fn page_columns(layouts: &[(u32, u32)], forced: u32, panel: (f64, f64)) -> u32 {
    let Some(&(first_w, _)) = layouts.first() else {
        return 1;
    };
    let count = |w: u32| ((w as f64 / first_w.max(1) as f64).round() as u32).max(1);
    if forced > 0 {
        return layouts
            .iter()
            .map(|l| count(l.0))
            .find(|n| *n == forced)
            .unwrap_or_else(|| count(layouts[(forced as usize).min(layouts.len()) - 1].0));
    }
    let (pw, ph) = panel;
    let mut best = 0.0;
    let mut at = 0;
    for (i, (w, h)) in layouts.iter().enumerate() {
        let s = (pw / *w as f64).min(ph / *h as f64);
        if s > best * 1.02 {
            best = s;
            at = i;
        }
    }
    count(layouts[at].0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_are_quarters() {
        assert_eq!(span_of(1.0), 4);
        assert_eq!(span_of(1.3), 5);
        assert_eq!(span_of(0.05), 1);
        assert_eq!(span_of(f64::NAN), 4);
        assert_eq!(span_of(50.0), MAX_SPAN);
        assert_eq!(weight_of(6), 1.5);
    }

    #[test]
    fn a_frame_stacks_its_groups() {
        let b = arrange(&[4, 8, 2], Flow::Down);
        assert_eq!((b.cols, b.rows), (1, 14));
        assert_eq!(
            b.blocks[1],
            Block {
                col: 0,
                row: 4,
                wide: 1,
                high: 8
            }
        );
        let b = arrange(&[4, 2], Flow::Across);
        assert_eq!((b.cols, b.rows), (6, 1));
        assert_eq!(
            b.blocks[1],
            Block {
                col: 4,
                row: 0,
                wide: 2,
                high: 1
            }
        );
    }

    #[test]
    fn page_columns_fill_as_css_grid_does() {
        // two columns: a tall group leaves room beside it for the next
        let b = arrange(&[8, 4, 4, 4], Flow::Columns(2));
        let at: Vec<(u32, u32)> = b.blocks.iter().map(|b| (b.col, b.row)).collect();
        assert_eq!(at, vec![(0, 0), (1, 0), (1, 4), (0, 8)]);
        assert_eq!(b.rows, 12);
        // the cursor never goes back: a short group after a tall one in the
        // last column starts on the next free row of the first column
        let b = arrange(&[4, 8, 4], Flow::Columns(2));
        let at: Vec<(u32, u32)> = b.blocks.iter().map(|b| (b.col, b.row)).collect();
        assert_eq!(at, vec![(0, 0), (1, 0), (0, 4)]);
    }

    #[test]
    fn the_page_picks_its_columns_by_the_panel() {
        let layouts = [(480, 1920), (960, 1060), (1440, 775), (2880, 450)];
        assert_eq!(page_columns(&layouts, 0, (480.0, 1920.0)), 1);
        assert_eq!(page_columns(&layouts, 0, (1920.0, 480.0)), 6);
        assert_eq!(page_columns(&layouts, 0, (1000.0, 1000.0)), 2);
        assert_eq!(page_columns(&layouts, 3, (480.0, 1920.0)), 3);
        assert_eq!(page_columns(&[], 0, (1.0, 1.0)), 1);
    }
}
