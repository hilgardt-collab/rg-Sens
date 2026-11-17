//! Layered drawing for combo panels.
//!
//! A plain `DrawingArea` is one render node: whenever anything in it changes,
//! GTK records the whole widget again, rasterises all of it on the CPU and
//! uploads the result. A combo panel with one item that changes ten times a
//! second (and glides in between) therefore paid for its frame and for every
//! other item on each animation frame; at 480x1920 that was most of a core.
//!
//! Here the panel is drawn as layers instead: the frame, each group title,
//! each content item. A layer comes with a key that covers everything its
//! drawing reads. While the key stays the same the render node made for it is
//! handed to GTK again, and GSK, which compares nodes by identity, neither
//! rasterises nor uploads that part of the panel. Only the layers whose key
//! changed are recorded again, each in a node as large as what it drew.
//!
//! The drawing code is the same for both targets: [`LayerSink::direct`] draws
//! every layer straight to a Cairo context (previews, tests),
//! [`LayerSink::cached`] goes through a [`LayerCache`].

use crate::core::PanelTransform;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{cairo, glib, graphene, gsk};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// Which layer of a panel: a kind and two indices (group, item).
pub type LayerId = (LayerKind, u32, u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerKind {
    /// Frame, backgrounds, dividers: what only the config and the size decide
    Chrome,
    /// A group's title bar
    Title,
    /// One content item
    Item,
    /// Effects drawn over the items
    Overlay,
}

/// A rectangle as `(x, y, width, height)`.
pub type Rect = (f64, f64, f64, f64);

/// Builds the key of a layer from the inputs of its drawing.
///
/// Every value the drawing reads must go in: a missed one shows as a layer
/// that keeps its old picture. `DefaultHasher::new()` uses fixed keys, so the
/// same inputs give the same key for the life of the process.
pub struct LayerKey(DefaultHasher);

impl Default for LayerKey {
    fn default() -> Self {
        Self::new()
    }
}

impl LayerKey {
    pub fn new() -> Self {
        Self(DefaultHasher::new())
    }

    pub fn u64(&mut self, value: u64) -> &mut Self {
        value.hash(&mut self.0);
        self
    }

    pub fn f64(&mut self, value: f64) -> &mut Self {
        value.to_bits().hash(&mut self.0);
        self
    }

    pub fn f64s(&mut self, values: &[f64]) -> &mut Self {
        values.len().hash(&mut self.0);
        for value in values {
            value.to_bits().hash(&mut self.0);
        }
        self
    }

    pub fn rect(&mut self, rect: Rect) -> &mut Self {
        self.f64s(&[rect.0, rect.1, rect.2, rect.3])
    }

    /// A map of values, whatever order it iterates in.
    pub fn values(&mut self, values: &HashMap<String, Value>) -> &mut Self {
        let mut sum = 0u64;
        for (name, value) in values {
            let mut entry = DefaultHasher::new();
            name.hash(&mut entry);
            hash_value(value, &mut entry);
            sum = sum.wrapping_add(entry.finish());
        }
        values.len().hash(&mut self.0);
        sum.hash(&mut self.0);
        self
    }

    pub fn finish(&self) -> u64 {
        self.0.finish()
    }
}

fn hash_value(value: &Value, state: &mut DefaultHasher) {
    match value {
        Value::Null => 0u8.hash(state),
        Value::Bool(b) => (1u8, b).hash(state),
        Value::Number(n) => {
            2u8.hash(state);
            if let Some(u) = n.as_u64() {
                (0u8, u).hash(state);
            } else if let Some(i) = n.as_i64() {
                (1u8, i).hash(state);
            } else {
                (2u8, n.as_f64().unwrap_or(0.0).to_bits()).hash(state);
            }
        }
        Value::String(s) => (3u8, s).hash(state),
        Value::Array(items) => {
            (4u8, items.len()).hash(state);
            for item in items {
                hash_value(item, state);
            }
        }
        Value::Object(map) => {
            (5u8, map.len()).hash(state);
            // serde_json keeps an object's keys in a fixed order
            for (name, item) in map {
                name.hash(state);
                hash_value(item, state);
            }
        }
    }
}

struct CachedLayer {
    key: u64,
    /// `None`: the layer drew nothing
    node: Option<gsk::RenderNode>,
    /// What the drawing returned (a title's height)
    result: f64,
    /// The pass that last used it
    pass: u64,
    /// How far the drawing reached when it was last measured, and when
    measured: Option<(Rect, Instant)>,
}

/// The render nodes of one panel's layers, kept between frames.
#[derive(Default)]
pub struct LayerCache {
    layers: HashMap<LayerId, CachedLayer>,
    pass: u64,
    /// The widget's size and the panel's transform the layers were made
    /// for. Both are drawn into a node, so another size or transform makes
    /// every layer anew.
    made_for: Option<[u64; 5]>,
    /// Layers recorded again, for tests and diagnostics
    pub redrawn: u64,
}

impl LayerCache {
    pub fn clear(&mut self) {
        self.layers.clear();
    }

    pub fn len(&self) -> usize {
        self.layers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
}

enum Target<'a> {
    Direct(&'a cairo::Context),
    Cached {
        snapshot: &'a gtk4::Snapshot,
        cache: &'a mut LayerCache,
        /// The widget's size: nothing is drawn outside it
        size: (f64, f64),
        /// The panel's transform as `(scale, translate_x, translate_y)`,
        /// applied to each layer's drawing
        transform: (f64, f64, f64),
    },
}

/// Where a panel's layers go.
pub struct LayerSink<'a> {
    target: Target<'a>,
    clip: Option<Rect>,
}

impl<'a> LayerSink<'a> {
    /// Every layer is drawn to `cr`, each time.
    pub fn direct(cr: &'a cairo::Context) -> Self {
        Self {
            target: Target::Direct(cr),
            clip: None,
        }
    }

    /// Layers go to `snapshot` as render nodes, kept in `cache` while their
    /// keys stay the same. The panel's transform is applied to each layer's
    /// drawing the way `PanelTransform::apply` applies it to a whole panel,
    /// so a layer draws, and is keyed, in the panel's own coordinates and
    /// its pixels are those of the direct drawing. Call
    /// [`finish`](Self::finish) after the last one.
    pub fn cached(
        snapshot: &'a gtk4::Snapshot,
        cache: &'a mut LayerCache,
        width: f64,
        height: f64,
        transform: &PanelTransform,
    ) -> Self {
        cache.pass = cache.pass.wrapping_add(1);
        let transform = transform.resolved();
        let made_for = [width, height, transform.0, transform.1, transform.2].map(f64::to_bits);
        if cache.made_for != Some(made_for) {
            cache.layers.clear();
            cache.made_for = Some(made_for);
        }
        Self {
            target: Target::Cached {
                snapshot,
                cache,
                size: (width, height),
                transform,
            },
            clip: None,
        }
    }

    /// Clip the layers that follow to `clip` (`None` lifts it).
    pub fn set_clip(&mut self, clip: Option<Rect>) {
        self.clip = clip;
    }

    /// Draw one layer. `key` must cover everything `draw` reads; `draw` may
    /// be called twice (once to measure, once for the node) or not at all,
    /// and what it returns is kept with the layer. `area` is where the layer
    /// is expected to draw (an item's box); it may draw outside it.
    pub fn layer(
        &mut self,
        id: LayerId,
        key: u64,
        area: Option<Rect>,
        draw: &dyn Fn(&cairo::Context) -> f64,
    ) -> f64 {
        let clip = self.clip;
        let clipped = |cr: &cairo::Context| -> f64 {
            cr.save().ok();
            if let Some((x, y, w, h)) = clip {
                cr.rectangle(x, y, w, h);
                cr.clip();
            }
            let result = draw(cr);
            cr.restore().ok();
            result
        };
        match &mut self.target {
            Target::Direct(cr) => clipped(cr),
            Target::Cached {
                snapshot,
                cache,
                size,
                transform,
            } => {
                let (width, height) = *size;
                let (scale, translate_x, translate_y) = *transform;
                // as `PanelTransform::apply`: shift, then scale about the centre
                let transformed = |cr: &cairo::Context| -> f64 {
                    cr.save().ok();
                    cr.translate(translate_x, translate_y);
                    if scale != 1.0 {
                        cr.translate(width / 2.0, height / 2.0);
                        cr.scale(scale, scale);
                        cr.translate(-width / 2.0, -height / 2.0);
                    }
                    let result = clipped(cr);
                    cr.restore().ok();
                    result
                };
                // the layer's area where it lands in the widget
                let area = area.map(|(x, y, w, h)| {
                    (
                        translate_x + width / 2.0 + scale * (x - width / 2.0),
                        translate_y + height / 2.0 + scale * (y - height / 2.0),
                        scale * w,
                        scale * h,
                    )
                });
                let limit = (0.0, 0.0, width, height);
                let pass = cache.pass;
                if let Some(layer) = cache.layers.get_mut(&id).filter(|l| l.key == key) {
                    layer.pass = pass;
                    if let Some(node) = &layer.node {
                        snapshot.append_node(node);
                    }
                    return layer.result;
                }
                cache.redrawn += 1;
                // A layer that changes many times a second (an item that
                // glides) is not measured for each frame: between two
                // measurements its node covers its area and what the last
                // one found. Whatever is drawn again less often than that is
                // measured every time.
                let now = Instant::now();
                let recent = cache
                    .layers
                    .get(&id)
                    .and_then(|layer| layer.measured)
                    .filter(|(_, at)| now.duration_since(*at) < REMEASURE_AFTER);
                let (bounds, measured, known) = match (recent, area) {
                    (Some((reach, at)), Some(area)) => (
                        pixel_bounds(union(area, reach), limit),
                        None,
                        Some((reach, at)),
                    ),
                    _ => {
                        let (reach, result) = measure(&transformed, limit);
                        let bounds = match (reach, area) {
                            (Reach::Nothing, _) => None,
                            (Reach::Unknown, _) => pixel_bounds(limit, limit),
                            // an item's node covers its box too, so that a
                            // frame drawn without measuring fits it
                            (Reach::To(reach), Some(area)) => {
                                pixel_bounds(union(area, reach), limit)
                            }
                            (Reach::To(reach), None) => pixel_bounds(reach, limit),
                        };
                        let known = match reach {
                            Reach::To(reach) => Some((reach, now)),
                            _ => None,
                        };
                        (bounds, Some(result), known)
                    }
                };
                let (node, result) = match bounds {
                    Some((x, y, w, h)) => {
                        let recorder = gtk4::Snapshot::new();
                        let cr = recorder.append_cairo(&graphene::Rect::new(
                            x as f32, y as f32, w as f32, h as f32,
                        ));
                        let result = transformed(&cr);
                        drop(cr);
                        (recorder.to_node(), result)
                    }
                    None => (None, measured.unwrap_or(0.0)),
                };
                if let Some(node) = &node {
                    snapshot.append_node(node);
                }
                cache.layers.insert(
                    id,
                    CachedLayer {
                        key,
                        node,
                        result,
                        pass,
                        measured: known,
                    },
                );
                result
            }
        }
    }

    /// Forget the layers this pass did not draw (items that are gone).
    pub fn finish(self) {
        if let Target::Cached { cache, .. } = self.target {
            let pass = cache.pass;
            cache.layers.retain(|_, layer| layer.pass == pass);
        }
    }
}

fn union(a: Rect, b: Rect) -> Rect {
    let x0 = a.0.min(b.0);
    let y0 = a.1.min(b.1);
    let x1 = (a.0 + a.2).max(b.0 + b.2);
    let y1 = (a.1 + a.3).max(b.1 + b.3);
    (x0, y0, x1 - x0, y1 - y0)
}

/// `rect` within `limit`, with a pixel of margin for antialiasing and out to
/// whole pixels; `None` if nothing is left.
fn pixel_bounds(rect: Rect, limit: Rect) -> Option<Rect> {
    let x0 = (rect.0 - 1.0).floor().max(limit.0.floor());
    let y0 = (rect.1 - 1.0).floor().max(limit.1.floor());
    let x1 = (rect.0 + rect.2 + 1.0)
        .ceil()
        .min((limit.0 + limit.2).ceil());
    let y1 = (rect.1 + rect.3 + 1.0)
        .ceil()
        .min((limit.1 + limit.3).ceil());
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1 - x0, y1 - y0))
}

/// A layer that was measured less than this long ago is drawn into its area
/// joined with what that measurement found.
const REMEASURE_AFTER: Duration = Duration::from_millis(250);

/// How far a drawing reached.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Reach {
    Nothing,
    To(Rect),
    /// It could not be measured
    Unknown,
}

/// Run the drawing on a recording surface to learn how far it reaches, so
/// the node is as large as what was drawn and no larger: glows and text that
/// leave an item's box keep showing, and a small item damages a small area.
/// The surface is as large as `limit` (what could show at all): on an
/// unbounded one a `paint()` without a clip has no extent and a frame that
/// starts with one would count as empty. Returns the reach and what the
/// drawing returned.
fn measure(draw: &dyn Fn(&cairo::Context) -> f64, limit: Rect) -> (Reach, f64) {
    let extents = cairo::Rectangle::new(
        limit.0.floor(),
        limit.1.floor(),
        (limit.0 + limit.2).ceil() - limit.0.floor(),
        (limit.1 + limit.3).ceil() - limit.1.floor(),
    );
    let Ok(surface) = cairo::RecordingSurface::create(cairo::Content::ColorAlpha, Some(extents))
    else {
        return (Reach::Unknown, 0.0);
    };
    let Ok(cr) = cairo::Context::new(&surface) else {
        return (Reach::Unknown, 0.0);
    };
    let result = draw(&cr);
    drop(cr);
    let (x, y, w, h) = surface.ink_extents();
    if !(x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite()) {
        return (Reach::Unknown, result);
    }
    if w <= 0.0 || h <= 0.0 {
        return (Reach::Nothing, result);
    }
    (Reach::To((x, y, w, h)), result)
}

type SnapshotFn = Box<dyn Fn(&gtk4::Snapshot, i32, i32)>;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct LayerCanvas {
        pub snapshot_fn: RefCell<Option<SnapshotFn>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LayerCanvas {
        const NAME: &'static str = "RgSensLayerCanvas";
        type Type = super::LayerCanvas;
        type ParentType = gtk4::DrawingArea;
    }

    impl ObjectImpl for LayerCanvas {
        fn dispose(&self) {
            // the closure holds the displayer's data
            self.snapshot_fn.borrow_mut().take();
        }
    }

    impl WidgetImpl for LayerCanvas {
        fn snapshot(&self, snapshot: &gtk4::Snapshot) {
            let widget = self.obj();
            let (width, height) = (widget.width(), widget.height());
            if width <= 0 || height <= 0 {
                return;
            }
            if let Some(draw) = self.snapshot_fn.borrow().as_ref() {
                draw(snapshot, width, height);
            }
        }
    }

    impl DrawingAreaImpl for LayerCanvas {}
}

glib::wrapper! {
    /// A `DrawingArea` whose content is built from render nodes by a closure
    /// (see the module documentation). It still is a `DrawingArea`, so the
    /// animation manager and everything else that holds one works as before.
    pub struct LayerCanvas(ObjectSubclass<imp::LayerCanvas>)
        @extends gtk4::DrawingArea, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl Default for LayerCanvas {
    fn default() -> Self {
        Self::new()
    }
}

impl LayerCanvas {
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Set what builds the content: called with the snapshot and the widget's
    /// size each time the widget is drawn.
    pub fn set_snapshot_func<F>(&self, draw: F)
    where
        F: Fn(&gtk4::Snapshot, i32, i32) + 'static,
    {
        *self.imp().snapshot_fn.borrow_mut() = Some(Box::new(draw));
        self.queue_draw();
    }
}

/// What the tests of the layered displayers share: a sequence of value
/// updates drawn both ways, which must give the same picture.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// Values for every slot. A slot's readings depend on `generation(i)`
    /// alone, so a caller decides which slots change from step to step.
    pub fn slot_values(
        prefixes: &[String],
        generation: &dyn Fn(usize) -> u64,
    ) -> HashMap<String, Value> {
        let mut values = HashMap::new();
        for (i, prefix) in prefixes.iter().enumerate() {
            let g = generation(i);
            let reading = ((i as u64 * 37 + g * 23) % 100) as f64 + 0.5;
            let mut put = |name: &str, value: Value| {
                values.insert(format!("{prefix}_{name}"), value);
            };
            put("caption", Value::from(format!("Item {i}")));
            put("value", Value::from(reading));
            put("numerical_value", Value::from(reading));
            put("unit", Value::from("%"));
            put("min_limit", Value::from(0.0));
            put("max_limit", Value::from(100.0));
            put("hour", Value::from((g % 12) as f64));
            put("minute", Value::from(((g * 7) % 60) as f64));
            put("second", Value::from(((g * 13) % 60) as f64));
            put(
                "time",
                Value::from(format!("12:{:02}:{:02}", (g * 7) % 60, (g * 13) % 60)),
            );
            for core in 0..32u64 {
                put(
                    &format!("core{core}_usage"),
                    Value::from(((core * 11 + g * 17 + i as u64) % 100) as f64),
                );
            }
        }
        values
    }

    fn pixels(surface: &cairo::ImageSurface) -> Vec<u8> {
        surface.flush();
        let mut out = Vec::new();
        surface
            .with_data(|data| out.extend_from_slice(data))
            .expect("surface data");
        out
    }

    /// Draw one step both ways and compare: `direct` to one node as large
    /// as the widget, which is what a plain drawing area makes of a draw
    /// function, `cached` as layers. Both go through a snapshot, because
    /// its Cairo contexts record (text is laid out a little differently on
    /// an image). `scaled` says the panel is under a scale, which allows a
    /// few more stray pixels (see below). Returns how many layers the cached
    /// way recorded again.
    pub fn same_picture(
        what: &str,
        scaled: bool,
        width: i32,
        height: i32,
        cache: &mut LayerCache,
        direct: &dyn Fn(&cairo::Context),
        cached: &dyn Fn(&gtk4::Snapshot, &mut LayerCache),
    ) -> u64 {
        let surface =
            || cairo::ImageSurface::create(cairo::Format::ARgb32, width, height).expect("surface");
        let draw_node = |snapshot: gtk4::Snapshot| {
            let surface = surface();
            if let Some(node) = snapshot.to_node() {
                let cr = cairo::Context::new(&surface).expect("context");
                node.draw(&cr);
            }
            surface
        };
        let whole = gtk4::Snapshot::new();
        {
            let cr =
                whole.append_cairo(&graphene::Rect::new(0.0, 0.0, width as f32, height as f32));
            direct(&cr);
        }
        let expected = draw_node(whole);
        let before = cache.redrawn;
        let snapshot = gtk4::Snapshot::new();
        cached(&snapshot, cache);
        let got = draw_node(snapshot);
        // LAYER_TEST_PNG=<dir> keeps both pictures to look at
        if let Ok(dir) = std::env::var("LAYER_TEST_PNG") {
            let name = what.replace([' ', ','], "_");
            for (kind, surface) in [("direct", &expected), ("layered", &got)] {
                if let Ok(mut file) = std::fs::File::create(format!("{dir}/{name}_{kind}.png")) {
                    surface.write_to_png(&mut file).ok();
                }
            }
        }
        let (expected, got) = (pixels(&expected), pixels(&got));
        // Layers are blended one over the other where the direct drawing
        // blends shape by shape, so translucent pixels (shadows, antialiased
        // edges) may round differently by a few levels. A layer that kept an
        // old picture, or is missing, differs by far more.
        const ROUNDING: u8 = 6;
        let differing = expected
            .chunks(4)
            .zip(got.chunks(4))
            .filter(|(a, b)| a.iter().zip(*b).any(|(x, y)| x.abs_diff(*y) > ROUNDING))
            .count();
        let worst = expected
            .iter()
            .zip(&got)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0);
        assert!(
            expected.iter().any(|&b| b != 0),
            "{what}: the direct drawing is empty"
        );
        // A few pixels may differ outright. Where the clip around the
        // content ends part of the way through a pixel row, Cairo antialiases
        // what hangs over that edge (the top of a graph's axis label) a
        // little differently when the item is replayed from a node of its
        // own. And under a scale, text centred exactly on half a pixel can
        // settle a row higher or lower, because the node's origin is taken
        // off its position before it is rounded; a node's origin is its
        // item's box, so that row does not change from frame to frame. A
        // stale item differs in far more pixels than either, and the steps
        // that would show one are not scaled.
        let stray = if scaled {
            (width * height) as usize / 500
        } else {
            32
        };
        assert!(
            differing <= stray,
            "{what}: {differing} pixels differ between the layered and the direct drawing (largest difference {worst})"
        );
        cache.redrawn - before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_does_not_depend_on_the_order_of_a_map() {
        let pairs = [("value", Value::from(1.5)), ("unit", Value::from("%"))];
        let forward: HashMap<String, Value> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        let backward: HashMap<String, Value> = pairs
            .iter()
            .rev()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        assert_eq!(
            LayerKey::new().values(&forward).finish(),
            LayerKey::new().values(&backward).finish()
        );
        let mut changed = forward.clone();
        changed.insert("value".to_string(), Value::from(1.6));
        assert_ne!(
            LayerKey::new().values(&forward).finish(),
            LayerKey::new().values(&changed).finish()
        );
    }

    #[test]
    fn measuring_finds_what_was_drawn() {
        let limit = (0.0, 0.0, 400.0, 300.0);
        let (reach, result) = measure(
            &|cr| {
                cr.rectangle(10.0, 20.0, 30.0, 40.0);
                cr.fill().ok();
                7.0
            },
            limit,
        );
        assert_eq!(reach, Reach::To((10.0, 20.0, 30.0, 40.0)));
        assert_eq!(result, 7.0);
        assert_eq!(
            pixel_bounds((10.0, 20.0, 30.0, 40.0), limit),
            Some((9.0, 19.0, 32.0, 42.0))
        );
        assert_eq!(measure(&|_| 0.0, limit).0, Reach::Nothing);
        // a paint without a clip reaches as far as anything can show (a
        // frame that starts by filling its background)
        let (reach, _) = measure(
            &|cr| {
                cr.set_source_rgb(0.0, 0.0, 0.0);
                cr.paint().ok();
                0.0
            },
            limit,
        );
        assert_eq!(reach, Reach::To(limit));
        // drawing that leaves the limit
        assert_eq!(
            pixel_bounds((-50.0, 250.0, 1000.0, 1000.0), limit),
            Some((0.0, 249.0, 400.0, 51.0))
        );
        assert_eq!(pixel_bounds((500.0, 0.0, 10.0, 10.0), limit), None);
        assert_eq!(
            union((0.0, 0.0, 10.0, 10.0), (5.0, -5.0, 10.0, 10.0)),
            (0.0, -5.0, 15.0, 15.0)
        );
    }
}
