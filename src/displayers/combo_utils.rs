//! Shared utilities for combo-style displayers (LCARS, Cyberpunk, Material, Industrial)
//!
//! This module provides optimized helper functions for:
//! - Extracting slot values from data hashmaps
//! - Parsing content item data
//! - Managing animation state
//! - Efficient prefix-based filtering and cleanup

use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::ui::graph_display::DataPoint;
use crate::ui::lcars_display::ContentItemData;

/// Animation state for a single value
#[derive(Debug, Clone)]
pub struct AnimatedValue {
    pub current: f64,
    pub target: f64,
    pub first_update: bool,
}

impl Default for AnimatedValue {
    fn default() -> Self {
        Self {
            current: 0.0,
            target: 0.0,
            first_update: true,
        }
    }
}

/// Pre-allocated key buffer for avoiding format! allocations in hot paths
pub struct KeyBuffer {
    buffer: String,
}

impl KeyBuffer {
    pub fn new() -> Self {
        Self {
            buffer: String::with_capacity(64),
        }
    }

    /// Build a key like "group1_2_caption" without allocating
    #[inline]
    pub fn build_field_key(&mut self, prefix: &str, suffix: &str) -> &str {
        self.buffer.clear();
        self.buffer.push_str(prefix);
        self.buffer.push('_');
        self.buffer.push_str(suffix);
        &self.buffer
    }

    /// Build a key like "group1_2" for prefix matching
    #[inline]
    pub fn build_prefix(&mut self, group_num: usize, item_idx: usize) -> &str {
        self.buffer.clear();
        use std::fmt::Write;
        let _ = write!(self.buffer, "group{}_{}", group_num, item_idx);
        &self.buffer
    }

    /// Build a bar key like "group1_2_bar" without allocating
    #[inline]
    pub fn build_bar_key(&mut self, prefix: &str) -> &str {
        self.buffer.clear();
        self.buffer.push_str(prefix);
        self.buffer.push_str("_bar");
        &self.buffer
    }

    /// Build a graph key like "group1_2_graph" without allocating
    #[inline]
    pub fn build_graph_key(&mut self, prefix: &str) -> &str {
        self.buffer.clear();
        self.buffer.push_str(prefix);
        self.buffer.push_str("_graph");
        &self.buffer
    }

    /// Build a core usage key like "group1_2_core0_usage" without allocating
    #[inline]
    pub fn build_core_key(&mut self, prefix: &str, core_idx: usize) -> &str {
        self.buffer.clear();
        use std::fmt::Write;
        let _ = write!(self.buffer, "{}_core{}_usage", prefix, core_idx);
        &self.buffer
    }

    /// Build a prefix with underscore like "group1_2_" for filtering
    #[inline]
    pub fn build_prefix_underscore(&mut self, prefix: &str) -> &str {
        self.buffer.clear();
        self.buffer.push_str(prefix);
        self.buffer.push('_');
        &self.buffer
    }

    /// Build an item prefix by appending item number to base prefix
    /// e.g., build_item_prefix("group1_", 1) -> "group1_1"
    #[inline]
    pub fn build_item_prefix(&mut self, base_prefix: &str, item_num: usize) -> &str {
        self.buffer.clear();
        use std::fmt::Write;
        let _ = write!(self.buffer, "{}{}", base_prefix, item_num);
        &self.buffer
    }
}

impl Default for KeyBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// Thread-local KeyBuffer to avoid allocations in hot paths
thread_local! {
    static KEY_BUFFER: std::cell::RefCell<KeyBuffer> = std::cell::RefCell::new(KeyBuffer::new());
}

/// Access the thread-local KeyBuffer for zero-allocation key building
#[inline]
pub fn with_key_buffer<F, R>(f: F) -> R
where
    F: FnOnce(&mut KeyBuffer) -> R,
{
    KEY_BUFFER.with(|buf| f(&mut buf.borrow_mut()))
}

/// Generate all prefixes for the given group item counts
/// Returns owned Strings since they're stored in collections
pub fn generate_prefixes(group_item_counts: &[usize]) -> Vec<String> {
    let total_items: usize = group_item_counts.iter().sum();
    let mut prefixes = Vec::with_capacity(total_items);

    KEY_BUFFER.with(|buf| {
        let mut key_buf = buf.borrow_mut();
        for (group_idx, &item_count) in group_item_counts.iter().enumerate() {
            let group_num = group_idx + 1;
            for item_idx in 1..=item_count {
                prefixes.push(key_buf.build_prefix(group_num, item_idx).to_string());
            }
        }
    });

    prefixes
}

/// Create a HashSet from prefixes for O(1) lookups
#[inline]
pub fn prefix_set(prefixes: &[String]) -> HashSet<&str> {
    prefixes.iter().map(|s| s.as_str()).collect()
}

/// Filter values to only those matching any of the given prefixes
/// Optimized single-pass algorithm: O(n) where n = data.len()
///
/// DEPRECATED: Use `filter_values_by_prefixes_into` for better performance
pub fn filter_values_by_prefixes(
    data: &HashMap<String, Value>,
    prefixes: &[String],
) -> HashMap<String, Value> {
    let mut result = HashMap::with_capacity(prefixes.len() * 8);
    filter_values_by_prefixes_into(data, prefixes, &mut result);
    result
}

/// Filter values in-place, reusing the output HashMap to avoid allocations
/// Clears `output` and fills it with matching values
#[inline]
pub fn filter_values_by_prefixes_into(
    data: &HashMap<String, Value>,
    prefixes: &[String],
    output: &mut HashMap<String, Value>,
) {
    let prefix_set = prefix_set(prefixes);
    filter_values_with_prefix_set(data, &prefix_set, output);
}

/// Filter values in-place using a pre-built prefix HashSet (borrowed &str version)
/// Use this variant when calling repeatedly with the same prefixes to avoid HashSet allocation
#[inline]
pub fn filter_values_with_prefix_set(
    data: &HashMap<String, Value>,
    prefix_set: &HashSet<&str>,
    output: &mut HashMap<String, Value>,
) {
    output.clear();

    // Single pass through data - O(n)
    for (k, v) in data.iter() {
        // Check if key matches any prefix exactly
        if prefix_set.contains(k.as_str()) {
            output.insert(k.clone(), v.clone());
            continue;
        }

        // Check if key starts with any prefix followed by underscore
        // Extract potential prefix from key (everything before first underscore after "group")
        if let Some(underscore_pos) = k.find('_') {
            if let Some(second_underscore) = k[underscore_pos + 1..].find('_') {
                let potential_prefix = &k[..underscore_pos + 1 + second_underscore];
                if prefix_set.contains(potential_prefix) {
                    output.insert(k.clone(), v.clone());
                }
            }
        }
    }
}

/// Filter values in-place using a pre-built prefix HashSet (owned String version)
/// Use this variant when you have a cached HashSet<String> to avoid repeated HashSet creation
#[inline]
pub fn filter_values_with_owned_prefix_set(
    data: &HashMap<String, Value>,
    prefix_set: &HashSet<String>,
    output: &mut HashMap<String, Value>,
) {
    output.clear();

    // Single pass through data - O(n)
    for (k, v) in data.iter() {
        // Check if key matches any prefix exactly
        if prefix_set.contains(k) {
            output.insert(k.clone(), v.clone());
            continue;
        }

        // Check if key starts with any prefix followed by underscore
        // Extract potential prefix from key (everything before first underscore after "group")
        if let Some(underscore_pos) = k.find('_') {
            if let Some(second_underscore) = k[underscore_pos + 1..].find('_') {
                let potential_prefix = &k[..underscore_pos + 1 + second_underscore];
                if prefix_set.contains(potential_prefix) {
                    output.insert(k.clone(), v.clone());
                }
            }
        }
    }
}

/// Get content item data from values with a given prefix
/// Optimized to minimize allocations
pub fn get_item_data(values: &HashMap<String, Value>, prefix: &str) -> ContentItemData {
    // Use a reusable buffer for key construction
    let mut key_buf = String::with_capacity(prefix.len() + 20);

    // Helper to build keys efficiently
    let mut make_key = |suffix: &str| -> String {
        key_buf.clear();
        key_buf.push_str(prefix);
        key_buf.push('_');
        key_buf.push_str(suffix);
        key_buf.clone()
    };

    let caption = values
        .get(&make_key("caption"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let value = values
        .get(&make_key("value"))
        .map(|v| match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    format!("{:.1}", f)
                } else {
                    n.to_string()
                }
            }
            _ => v.to_string(),
        })
        .unwrap_or_default();

    let unit = values
        .get(&make_key("unit"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Try numerical_value first, fall back to value
    let numerical_value_key = make_key("numerical_value");
    let value_key = make_key("value");
    let numerical_value = values
        .get(&numerical_value_key)
        .or_else(|| values.get(&value_key))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let min_value = values
        .get(&make_key("min_limit"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let max_value = values
        .get(&make_key("max_limit"))
        .and_then(|v| v.as_f64())
        .unwrap_or(100.0);

    ContentItemData {
        caption,
        value,
        unit,
        numerical_value,
        min_value,
        max_value,
    }
}

/// Get all values for a slot with the prefix stripped
/// Optimized to avoid format! allocation
pub fn get_slot_values(values: &HashMap<String, Value>, prefix: &str) -> HashMap<String, Value> {
    let prefix_len = prefix.len() + 1; // +1 for underscore

    KEY_BUFFER.with(|buf| {
        let mut key_buf = buf.borrow_mut();
        let prefix_with_underscore = key_buf.build_prefix_underscore(prefix);

        values.iter()
            .filter(|(k, _)| k.starts_with(prefix_with_underscore))
            .map(|(k, v)| {
                let short_key = &k[prefix_len..];
                (short_key.to_string(), v.clone())
            })
            .collect()
    })
}

/// Update bar animation state - optimized version using thread-local KeyBuffer
#[inline]
pub fn update_bar_animation(
    bar_values: &mut HashMap<String, AnimatedValue>,
    prefix: &str,
    target_percent: f64,
    animation_enabled: bool,
) {
    // Build key using thread-local buffer, then convert to owned String only if needed
    let bar_key = KEY_BUFFER.with(|buf| {
        let mut key_buf = buf.borrow_mut();
        let key = key_buf.build_bar_key(prefix);
        // Only allocate if key doesn't exist
        if bar_values.contains_key(key) {
            None
        } else {
            Some(key.to_string())
        }
    });

    let anim = if let Some(new_key) = bar_key {
        bar_values.entry(new_key).or_default()
    } else {
        // Key exists - look it up again (cheap compared to allocation)
        KEY_BUFFER.with(|buf| {
            let mut key_buf = buf.borrow_mut();
            let key = key_buf.build_bar_key(prefix);
            // Use expect with context - this should never fail since we just checked contains_key
            bar_values.get_mut(key).expect("bar key disappeared between contains_key check and get_mut")
        })
    };

    // Only update target if the change is visually significant (> 0.5%)
    // This prevents constant 60fps animation from tiny sensor fluctuations
    const TARGET_CHANGE_THRESHOLD: f64 = 0.005;
    let target_changed = (anim.target - target_percent).abs() > TARGET_CHANGE_THRESHOLD;

    if target_changed {
        anim.target = target_percent;
    }

    if anim.first_update || !animation_enabled {
        anim.current = target_percent;
        anim.first_update = false;
    }
}

/// Update bar animation target, returning true if the value changed meaningfully
pub fn update_bar_animation_with_change_detection(
    bar_values: &mut HashMap<String, AnimatedValue>,
    prefix: &str,
    target_percent: f64,
    animation_enabled: bool,
) -> bool {
    let bar_key = KEY_BUFFER.with(|buf| {
        let mut key_buf = buf.borrow_mut();
        let key = key_buf.build_bar_key(prefix);
        if bar_values.contains_key(key) {
            None
        } else {
            Some(key.to_string())
        }
    });

    let anim = if let Some(new_key) = bar_key {
        bar_values.entry(new_key).or_default()
    } else {
        KEY_BUFFER.with(|buf| {
            let mut key_buf = buf.borrow_mut();
            let key = key_buf.build_bar_key(prefix);
            bar_values.get_mut(key).expect("bar key disappeared")
        })
    };

    const TARGET_CHANGE_THRESHOLD: f64 = 0.005;
    let target_changed = (anim.target - target_percent).abs() > TARGET_CHANGE_THRESHOLD;

    if target_changed {
        anim.target = target_percent;
    }

    if anim.first_update || !animation_enabled {
        anim.current = target_percent;
        anim.first_update = false;
        return true; // First update always counts as a change
    }

    target_changed
}

/// Update graph history - optimized version using thread-local KeyBuffer
pub fn update_graph_history(
    graph_history: &mut HashMap<String, VecDeque<DataPoint>>,
    prefix: &str,
    numerical_value: f64,
    timestamp: f64,
    max_points: usize,
) {
    // Build key using thread-local buffer
    let graph_key = KEY_BUFFER.with(|buf| {
        let mut key_buf = buf.borrow_mut();
        let key = key_buf.build_graph_key(prefix);
        if graph_history.contains_key(key) {
            None
        } else {
            Some(key.to_string())
        }
    });

    let history = if let Some(new_key) = graph_key {
        graph_history.entry(new_key).or_default()
    } else {
        KEY_BUFFER.with(|buf| {
            let mut key_buf = buf.borrow_mut();
            let key = key_buf.build_graph_key(prefix);
            // Use expect with context - this should never fail since we just checked contains_key
            graph_history.get_mut(key).expect("graph key disappeared between contains_key check and get_mut")
        })
    };

    history.push_back(DataPoint {
        value: numerical_value,
        timestamp,
    });

    while history.len() > max_points {
        history.pop_front();
    }
}

/// Update core bars animation state - optimized to reduce allocations
pub fn update_core_bars(
    data: &HashMap<String, Value>,
    core_bar_values: &mut HashMap<String, Vec<AnimatedValue>>,
    prefix: &str,
    config: &crate::ui::core_bars_display::CoreBarsConfig,
    animation_enabled: bool,
) {
    let capacity = config.end_core.saturating_sub(config.start_core) + 1;
    let mut core_targets: Vec<f64> = Vec::with_capacity(capacity);

    // Try configured core range first - use KeyBuffer for core keys
    KEY_BUFFER.with(|buf| {
        let mut key_buf = buf.borrow_mut();
        for core_idx in config.start_core..=config.end_core {
            let core_key = key_buf.build_core_key(prefix, core_idx);
            let value = data.get(core_key)
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) / 100.0;
            core_targets.push(value);
        }
    });

    // Auto-detect cores if configured range found nothing
    if core_targets.is_empty() || core_targets.iter().all(|&v| v == 0.0) {
        core_targets.clear();
        KEY_BUFFER.with(|buf| {
            let mut key_buf = buf.borrow_mut();
            for core_idx in 0..128 {
                let core_key = key_buf.build_core_key(prefix, core_idx);
                if let Some(v) = data.get(core_key).and_then(|v| v.as_f64()) {
                    core_targets.push(v / 100.0);
                } else if core_idx > 0 {
                    break;
                }
            }
        });
    }

    let anims = core_bar_values.entry(prefix.to_string()).or_default();

    // Resize animation vector to match core count
    while anims.len() < core_targets.len() {
        anims.push(AnimatedValue::default());
    }
    anims.truncate(core_targets.len());

    // Update animation targets (only if change is visually significant)
    const TARGET_CHANGE_THRESHOLD: f64 = 0.005;
    for (i, &target) in core_targets.iter().enumerate() {
        if let Some(anim) = anims.get_mut(i) {
            // Only update target if the change is significant (> 0.5%)
            let target_changed = (anim.target - target).abs() > TARGET_CHANGE_THRESHOLD;
            if target_changed {
                anim.target = target;
            }
            if anim.first_update || !animation_enabled {
                anim.current = target;
                anim.first_update = false;
            }
        }
    }
}

/// Clean up all stale animation entries in one pass
/// Builds the prefix HashSet once and cleans up all collections
#[inline]
pub fn cleanup_all_animation_state(
    bar_values: &mut HashMap<String, AnimatedValue>,
    core_bar_values: &mut HashMap<String, Vec<AnimatedValue>>,
    graph_history: &mut HashMap<String, VecDeque<DataPoint>>,
    prefixes: &[String],
) {
    // Build prefix set once - O(n) where n = prefixes.len()
    let prefix_set: HashSet<&str> = prefixes.iter().map(|s| s.as_str()).collect();

    // Clean up bar values
    bar_values.retain(|k, _| {
        k.strip_suffix("_bar")
            .map(|p| prefix_set.contains(p))
            .unwrap_or(false)
    });

    // Clean up core bar values
    core_bar_values.retain(|k, _| prefix_set.contains(k.as_str()));

    // Clean up graph history
    graph_history.retain(|k, _| {
        k.strip_suffix("_graph")
            .map(|p| prefix_set.contains(p))
            .unwrap_or(false)
    });
}

/// Clean up stale bar animation entries using retain
/// Prefer cleanup_all_animation_state when cleaning multiple collections
#[inline]
pub fn cleanup_bar_values(bar_values: &mut HashMap<String, AnimatedValue>, prefixes: &[String]) {
    let prefix_set: HashSet<&str> = prefixes.iter().map(|s| s.as_str()).collect();
    bar_values.retain(|k, _| {
        k.strip_suffix("_bar")
            .map(|p| prefix_set.contains(p))
            .unwrap_or(false)
    });
}

/// Clean up stale core bar animation entries using retain
/// Prefer cleanup_all_animation_state when cleaning multiple collections
#[inline]
pub fn cleanup_core_bar_values(core_bar_values: &mut HashMap<String, Vec<AnimatedValue>>, prefixes: &[String]) {
    let prefix_set: HashSet<&str> = prefixes.iter().map(|s| s.as_str()).collect();
    core_bar_values.retain(|k, _| prefix_set.contains(k.as_str()));
}

/// Clean up stale graph history entries using retain
/// Prefer cleanup_all_animation_state when cleaning multiple collections
#[inline]
pub fn cleanup_graph_history(graph_history: &mut HashMap<String, VecDeque<DataPoint>>, prefixes: &[String]) {
    let prefix_set: HashSet<&str> = prefixes.iter().map(|s| s.as_str()).collect();
    graph_history.retain(|k, _| {
        k.strip_suffix("_graph")
            .map(|p| prefix_set.contains(p))
            .unwrap_or(false)
    });
}

/// Process animation frame - interpolate all animated values toward targets
/// Returns true if any value changed (needs redraw)
pub fn animate_values(
    bar_values: &mut HashMap<String, AnimatedValue>,
    core_bar_values: &mut HashMap<String, Vec<AnimatedValue>>,
    animation_speed: f64,
    snap_threshold: f64,
) -> bool {
    let mut needs_redraw = false;
    let delta = animation_speed * 0.016; // ~60fps frame time

    for anim in bar_values.values_mut() {
        let diff = anim.target - anim.current;
        if diff.abs() > snap_threshold {
            anim.current += diff * delta;
            needs_redraw = true;
        } else if (anim.current - anim.target).abs() > f64::EPSILON {
            anim.current = anim.target;
            needs_redraw = true;
        }
    }

    for core_anims in core_bar_values.values_mut() {
        for anim in core_anims.iter_mut() {
            let diff = anim.target - anim.current;
            if diff.abs() > snap_threshold {
                anim.current += diff * delta;
                needs_redraw = true;
            } else if (anim.current - anim.target).abs() > f64::EPSILON {
                anim.current = anim.target;
                needs_redraw = true;
            }
        }
    }

    needs_redraw
}

