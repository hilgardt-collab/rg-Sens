//! Widget for selecting position from a 3x3 grid with arrow icons

use crate::displayers::TextPosition;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Orientation, ToggleButton};
use std::cell::RefCell;
use std::rc::Rc;

/// Labels for each position using Unicode arrows
const POSITION_LABELS: [[&str; 3]; 3] = [
    ["↖", "↑", "↗"], // Top row
    ["←", "●", "→"], // Center row
    ["↙", "↓", "↘"], // Bottom row
];

/// TextPosition values for each grid cell
const POSITIONS: [[TextPosition; 3]; 3] = [
    [TextPosition::TopLeft, TextPosition::TopCenter, TextPosition::TopRight],
    [TextPosition::CenterLeft, TextPosition::Center, TextPosition::CenterRight],
    [TextPosition::BottomLeft, TextPosition::BottomCenter, TextPosition::BottomRight],
];

/// Widget for selecting position from a 3x3 grid
#[allow(dead_code)]
pub struct PositionGridWidget {
    container: GtkBox,
    buttons: [[ToggleButton; 3]; 3],
    position: Rc<RefCell<TextPosition>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn(TextPosition)>>>>,
    updating: Rc<RefCell<bool>>, // Guard flag to prevent recursion
}

impl PositionGridWidget {
    /// Create a new position grid widget with the given initial position
    pub fn new(initial: TextPosition) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);
        container.add_css_class("position-grid");

        let position = Rc::new(RefCell::new(initial));
        let on_change: Rc<RefCell<Option<Box<dyn Fn(TextPosition)>>>> = Rc::new(RefCell::new(None));
        let updating = Rc::new(RefCell::new(false)); // Guard flag

        // Create all buttons first
        let mut buttons_temp: [[Option<ToggleButton>; 3]; 3] = Default::default();

        for row in 0..3 {
            let row_box = GtkBox::new(Orientation::Horizontal, 0);
            for col in 0..3 {
                let btn = ToggleButton::with_label(POSITION_LABELS[row][col]);
                btn.set_size_request(28, 28);
                btn.add_css_class("position-grid-button");

                // Make buttons look like a grid
                if row == 0 && col == 0 {
                    btn.add_css_class("top-left");
                } else if row == 0 && col == 2 {
                    btn.add_css_class("top-right");
                } else if row == 2 && col == 0 {
                    btn.add_css_class("bottom-left");
                } else if row == 2 && col == 2 {
                    btn.add_css_class("bottom-right");
                }

                let is_selected = POSITIONS[row][col] == initial;
                btn.set_active(is_selected);

                row_box.append(&btn);
                buttons_temp[row][col] = Some(btn);
            }
            container.append(&row_box);
        }

        // Convert to non-optional array
        let buttons: [[ToggleButton; 3]; 3] = [
            [
                buttons_temp[0][0].take().unwrap(),
                buttons_temp[0][1].take().unwrap(),
                buttons_temp[0][2].take().unwrap(),
            ],
            [
                buttons_temp[1][0].take().unwrap(),
                buttons_temp[1][1].take().unwrap(),
                buttons_temp[1][2].take().unwrap(),
            ],
            [
                buttons_temp[2][0].take().unwrap(),
                buttons_temp[2][1].take().unwrap(),
                buttons_temp[2][2].take().unwrap(),
            ],
        ];

        // Connect click handlers for mutual exclusion
        for row in 0..3 {
            for col in 0..3 {
                let pos = POSITIONS[row][col];
                let position_clone = position.clone();
                let on_change_clone = on_change.clone();
                let updating_clone = updating.clone();

                // Clone all buttons for the closure
                let buttons_clone: [[ToggleButton; 3]; 3] = [
                    [buttons[0][0].clone(), buttons[0][1].clone(), buttons[0][2].clone()],
                    [buttons[1][0].clone(), buttons[1][1].clone(), buttons[1][2].clone()],
                    [buttons[2][0].clone(), buttons[2][1].clone(), buttons[2][2].clone()],
                ];

                let current_row = row;
                let current_col = col;

                buttons[row][col].connect_toggled(move |btn| {
                    // Skip if we're already updating (prevents recursion)
                    if *updating_clone.borrow() {
                        return;
                    }

                    if btn.is_active() {
                        // Set guard flag
                        *updating_clone.borrow_mut() = true;

                        // Deactivate all other buttons
                        #[allow(clippy::needless_range_loop)]
                        for r in 0..3 {
                            for c in 0..3 {
                                if r != current_row || c != current_col {
                                    buttons_clone[r][c].set_active(false);
                                }
                            }
                        }

                        // Clear guard flag
                        *updating_clone.borrow_mut() = false;

                        *position_clone.borrow_mut() = pos;

                        if let Some(ref callback) = *on_change_clone.borrow() {
                            callback(pos);
                        }
                    } else {
                        // Don't allow complete deselection - reactivate this button
                        // Set guard to prevent the re-activation from triggering the handler
                        *updating_clone.borrow_mut() = true;
                        btn.set_active(true);
                        *updating_clone.borrow_mut() = false;
                    }
                });
            }
        }

        Self {
            container,
            buttons,
            position,
            on_change,
            updating,
        }
    }

    /// Get the widget container
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    /// Get the currently selected position
    #[allow(dead_code)]
    pub fn position(&self) -> TextPosition {
        *self.position.borrow()
    }

    /// Set the selected position programmatically
    #[allow(dead_code)]
    pub fn set_position(&self, pos: TextPosition) {
        *self.position.borrow_mut() = pos;

        // Set guard flag to prevent callback during programmatic update
        *self.updating.borrow_mut() = true;

        // Update button states
        #[allow(clippy::needless_range_loop)]
        for row in 0..3 {
            for col in 0..3 {
                let should_be_active = POSITIONS[row][col] == pos;
                self.buttons[row][col].set_active(should_be_active);
            }
        }

        // Clear guard flag
        *self.updating.borrow_mut() = false;
    }

    /// Set the callback for when the position changes
    pub fn set_on_change<F: Fn(TextPosition) + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }
}
