//! Shared constants for the application

use std::time::Duration;

/// Animation frame interval for smooth 60fps animations (16ms)
pub const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Animation frame interval in milliseconds (useful for calculations)
pub const ANIMATION_FRAME_MS: u64 = 16;

/// Polling interval for static (non-animated) displayers to check dirty flag (250ms)
/// This is less frequent than animation frames since data sources typically update
/// every 1 second, so checking 4 times per second is sufficient while reducing CPU usage.
pub const STATIC_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Threshold for snapping animation values to their target.
/// When the difference between animated and target values is less than this,
/// the animation is considered complete and snaps to the target.
/// This represents 0.1% precision, suitable for 0.0-1.0 normalized values.
pub const ANIMATION_SNAP_THRESHOLD: f64 = 0.001;

/// Threshold for detecting meaningful transform changes (scale, translate).
/// Values smaller than this are considered effectively zero/unchanged.
/// Using 0.001 instead of f64::EPSILON for practical UI-level precision.
pub const TRANSFORM_THRESHOLD: f64 = 0.001;

// Memory unit conversion constants (bytes to larger units)

/// Bytes per kilobyte (1024)
pub const BYTES_PER_KB: f64 = 1024.0;

/// Bytes per megabyte (1024^2)
pub const BYTES_PER_MB: f64 = 1024.0 * 1024.0;

/// Bytes per gigabyte (1024^3)
pub const BYTES_PER_GB: f64 = 1024.0 * 1024.0 * 1024.0;

/// Bytes per terabyte (1024^4)
pub const BYTES_PER_TB: f64 = 1024.0 * 1024.0 * 1024.0 * 1024.0;
