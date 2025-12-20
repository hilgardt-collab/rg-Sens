//! Shared constants for the application

use std::time::Duration;

/// Animation frame interval for smooth 60fps animations (16ms)
pub const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Animation frame interval in milliseconds (useful for calculations)
pub const ANIMATION_FRAME_MS: u64 = 16;

/// Threshold for snapping animation values to their target.
/// When the difference between animated and target values is less than this,
/// the animation is considered complete and snaps to the target.
/// This represents 0.1% precision, suitable for 0.0-1.0 normalized values.
pub const ANIMATION_SNAP_THRESHOLD: f64 = 0.001;
