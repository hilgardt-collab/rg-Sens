//! Field editors for `PropKind::Custom` props: controls a plain schema kind
//! cannot express. Each is registered by name in [`register_field_editors`]
//! and receives a [`FieldContext`] to write the flat config through (silent
//! while the editor loads, so normalising a saved value does not commit).

use super::schema_editor::{register_field_editor, FieldContext, FieldEditor, FieldRefresh};
use crate::sources::{
    describe_sensor_selection, resolve_fan_index, resolve_sensor_index, set_fan_by_index,
    set_sensor_by_index, CpuSource, FanSpeedConfig, FanSpeedSource, SystemTempConfig,
    SystemTempSource,
};
use crate::ui::position_grid_widget::PositionGridWidget;
use crate::ui::sensor_picker::SensorPicker;
use crate::ui::{BackgroundConfigWidget, TimezoneDialog};
use gtk4::prelude::*;
use gtk4::{Button, DropDown, StringList};
use rg_sens_types::background::BackgroundConfig;
use rg_sens_types::display_configs::IndicatorShape;
use rg_sens_types::text::TextPosition;
use serde_json::{json, Map, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Register every built-in field editor. Called from `register_all_editors`.
pub fn register_field_editors() {
    register_field_editor("timezone", timezone_editor);
    register_field_editor("fan_sensor", fan_sensor_editor);
    register_field_editor("temp_sensor", temp_sensor_editor);
    register_field_editor("cpu_core_selection", cpu_core_editor);
    register_field_editor("text_position", text_position_editor);
    register_field_editor("background", background_editor);
    register_field_editor("indicator_shape", indicator_shape_editor);
}

/// A button showing the zone name; clicking opens the timezone dialog.
fn timezone_editor(ctx: FieldContext) -> FieldEditor {
    let key = ctx.prop.key;
    let button = Button::with_label("Local");
    let current = Rc::new(RefCell::new("Local".to_string()));
    {
        let ctx = ctx.clone();
        let current = current.clone();
        button.connect_clicked(move |btn| {
            let ctx = ctx.clone();
            let current = current.clone();
            let btn = btn.clone();
            let now = current.borrow().clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                let window = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok());
                if let Some(tz) = TimezoneDialog::pick_timezone(window.as_ref(), &now).await {
                    btn.set_label(&tz);
                    *current.borrow_mut() = tz.clone();
                    ctx.set(vec![(key, Value::String(tz))]);
                }
            });
        });
    }
    let refresh: FieldRefresh = {
        let button = button.clone();
        Box::new(move |m: &Map<String, Value>| {
            let tz = m
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("Local")
                .to_string();
            button.set_label(&tz);
            *current.borrow_mut() = tz;
        })
    };
    FieldEditor::new(button, refresh)
}

fn fan_sensor_editor(ctx: FieldContext) -> FieldEditor {
    let labels: Vec<String> = FanSpeedSource::available_sensors()
        .iter()
        .map(|s| format!("[{:?}] {}", s.category, s.label))
        .collect();
    sensor_editor(
        ctx,
        "fan",
        labels,
        |index| {
            let mut cfg = FanSpeedConfig::default();
            set_fan_by_index(&mut cfg, index);
            (cfg.sensor_label, cfg.sensor_index)
        },
        |label, index| {
            let mut cfg = FanSpeedConfig {
                sensor_label: label,
                sensor_index: index,
                ..Default::default()
            };
            let resolved = resolve_fan_index(&mut cfg);
            (resolved, cfg.sensor_label, cfg.sensor_index)
        },
    )
}

fn temp_sensor_editor(ctx: FieldContext) -> FieldEditor {
    let labels: Vec<String> = SystemTempSource::available_sensors()
        .iter()
        .map(|s| format!("[{:?}] {}", s.category, s.label))
        .collect();
    sensor_editor(
        ctx,
        "sensor",
        labels,
        |index| {
            let mut cfg = SystemTempConfig::default();
            set_sensor_by_index(&mut cfg, index);
            (cfg.sensor_label, cfg.sensor_index)
        },
        |label, index| {
            let mut cfg = SystemTempConfig {
                sensor_label: label,
                sensor_index: index,
                ..Default::default()
            };
            let resolved = resolve_sensor_index(&mut cfg);
            (resolved, cfg.sensor_label, cfg.sensor_index)
        },
    )
}

type Picked = (Option<String>, usize);
type Resolved = (Option<usize>, Option<String>, usize);

/// The sensor picker over `sensor_label` + `sensor_index` (gotcha 18: the
/// label is the identity, the index a legacy fallback). On load the saved
/// pair is resolved against this boot's sensors; a match is written back in
/// normalised form (label filled in, index current), no match shows the
/// placeholder row and a warning and leaves the saved pair untouched.
fn sensor_editor(
    ctx: FieldContext,
    kind: &'static str,
    labels: Vec<String>,
    pick: impl Fn(usize) -> Picked + 'static,
    resolve: impl Fn(Option<String>, usize) -> Resolved + 'static,
) -> FieldEditor {
    let write: Rc<dyn Fn(Option<String>, usize)> = Rc::new(move |label, index| {
        ctx.set(vec![
            ("sensor_label", json!(label)),
            ("sensor_index", json!(index)),
        ])
    });
    let picker = Rc::new(SensorPicker::new(kind, &labels, {
        let write = write.clone();
        move |index| {
            let (label, idx) = pick(index);
            write(label, idx);
        }
    }));
    let refresh: FieldRefresh = {
        let picker = picker.clone();
        Box::new(move |m: &Map<String, Value>| {
            let label = m
                .get("sensor_label")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let index = m.get("sensor_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let (resolved, norm_label, norm_index) = resolve(label.clone(), index);
            picker.set_resolved(
                resolved,
                &describe_sensor_selection(label.as_deref(), index),
            );
            if resolved.is_some() {
                write(norm_label, norm_index);
            }
        })
    };
    FieldEditor::new(picker.widget().clone(), refresh)
}

/// `CoreSelection` serialises as `"Overall"` or `{"Core": n}`; a dropdown of
/// "Overall, Core 0 … Core N-1" over this machine's core count. A saved core
/// past the count keeps its own row.
fn cpu_core_editor(ctx: FieldContext) -> FieldEditor {
    let key = ctx.prop.key;
    let count = CpuSource::get_cached_core_count();
    let dropdown = DropDown::new(None::<StringList>, None::<gtk4::Expression>);
    let values: Rc<RefCell<Vec<Value>>> = Rc::new(RefCell::new(Vec::new()));
    let refreshing = Rc::new(Cell::new(false));
    {
        let values = values.clone();
        let refreshing = refreshing.clone();
        dropdown.connect_selected_notify(move |d| {
            if refreshing.get() {
                return;
            }
            if let Some(v) = values.borrow().get(d.selected() as usize) {
                ctx.set(vec![(key, v.clone())]);
            }
        });
    }
    let refresh: FieldRefresh = {
        let dropdown = dropdown.clone();
        Box::new(move |m: &Map<String, Value>| {
            let saved_core = m
                .get(key)
                .and_then(|v| v.get("Core"))
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            let mut labels = vec!["Overall".to_string()];
            let mut vals = vec![json!("Overall")];
            for i in 0..count {
                labels.push(format!("Core {}", i));
                vals.push(json!({ "Core": i }));
            }
            if let Some(c) = saved_core.filter(|c| *c >= count) {
                labels.push(format!("Core {} (not present)", c));
                vals.push(json!({ "Core": c }));
            }
            let idx = match saved_core {
                None => 0,
                Some(c) => vals
                    .iter()
                    .position(|v| v.get("Core").and_then(|n| n.as_u64()) == Some(c as u64))
                    .unwrap_or(0),
            };
            let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            *values.borrow_mut() = vals;
            refreshing.set(true);
            dropdown.set_model(Some(&StringList::new(&refs)));
            dropdown.set_selected(idx as u32);
            refreshing.set(false);
        })
    };
    FieldEditor::new(dropdown, refresh)
}

/// A 3×3 position grid over a `TextPosition` (`set_position` is guarded).
fn text_position_editor(ctx: FieldContext) -> FieldEditor {
    let key = ctx.prop.key;
    let grid = Rc::new(PositionGridWidget::new(TextPosition::Center));
    {
        let ctx = ctx.clone();
        grid.set_on_change(move |pos| {
            if let Ok(v) = serde_json::to_value(pos) {
                ctx.set(vec![(key, v)]);
            }
        });
    }
    let refresh: FieldRefresh = {
        let grid = grid.clone();
        Box::new(move |m: &Map<String, Value>| {
            if let Some(pos) = m
                .get(key)
                .and_then(|v| serde_json::from_value::<TextPosition>(v.clone()).ok())
            {
                grid.set_position(pos);
            }
        })
    };
    let g = grid.clone();
    FieldEditor::new(grid.widget().clone(), refresh).with_cleanup(move || g.cleanup())
}

/// The full background editor (solid / gradients / image / polygons /
/// indicator) over a `BackgroundConfig`, with the slot's fields for the
/// indicator type. `set_config` drives a type dropdown whose handlers fire
/// the widget's change signal, so writes are suppressed while refreshing.
fn background_editor(ctx: FieldContext) -> FieldEditor {
    let key = ctx.prop.key;
    let widget = Rc::new(BackgroundConfigWidget::new());
    widget.set_source_fields(ctx.fields.to_vec());
    widget.set_theme_config(ctx.theme.clone());
    let refreshing = Rc::new(Cell::new(false));
    {
        let ctx = ctx.clone();
        let w = widget.clone();
        let refreshing = refreshing.clone();
        widget.set_on_change(move || {
            if refreshing.get() {
                return;
            }
            if let Ok(v) = serde_json::to_value(w.get_config()) {
                ctx.set(vec![(key, v)]);
            }
        });
    }
    let refresh: FieldRefresh = {
        let w = widget.clone();
        Box::new(move |m: &Map<String, Value>| {
            if let Some(cfg) = m
                .get(key)
                .and_then(|v| serde_json::from_value::<BackgroundConfig>(v.clone()).ok())
            {
                refreshing.set(true);
                w.set_config(cfg);
                refreshing.set(false);
            }
        })
    };
    let w_theme = widget.clone();
    let w_cleanup = widget.clone();
    FieldEditor::new(widget.widget().clone(), refresh)
        .as_block()
        .with_theme(move |t| w_theme.set_theme_config(t.clone()))
        // The change closure holds the widget (gotcha 11): drop it with the row.
        .with_cleanup(move || w_cleanup.set_on_change(|| {}))
}

/// `IndicatorShape` is externally tagged with a payload (`"circle"`,
/// `{"polygon": 5}`): a dropdown of the common shapes plus a custom
/// side count.
fn indicator_shape_editor(ctx: FieldContext) -> FieldEditor {
    const PRESETS: [(&str, Option<u32>); 8] = [
        ("Fill", None),
        ("Circle", None),
        ("Square", None),
        ("Triangle", Some(3)),
        ("Pentagon", Some(5)),
        ("Hexagon", Some(6)),
        ("Heptagon", Some(7)),
        ("Octagon", Some(8)),
    ];
    let key = ctx.prop.key;
    let labels: Vec<&str> = PRESETS
        .iter()
        .map(|(l, _)| *l)
        .chain(std::iter::once("Custom polygon"))
        .collect();
    let custom_index = PRESETS.len() as u32;
    let inner = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let dropdown = DropDown::new(Some(StringList::new(&labels)), None::<gtk4::Expression>);
    dropdown.set_hexpand(true);
    let sides = gtk4::SpinButton::with_range(3.0, 64.0, 1.0);
    sides.set_tooltip_text(Some("Sides"));
    sides.set_visible(false);
    inner.append(&dropdown);
    inner.append(&sides);
    let refreshing = Rc::new(Cell::new(false));
    let shape_value = {
        let sides = sides.clone();
        move |index: u32| -> Value {
            match index {
                0 => json!("fill"),
                1 => json!("circle"),
                2 => json!("square"),
                i if (i as usize) < PRESETS.len() => {
                    json!({ "polygon": PRESETS[i as usize].1.unwrap_or(3) })
                }
                _ => json!({ "polygon": sides.value() as u32 }),
            }
        }
    };
    {
        let ctx = ctx.clone();
        let sides = sides.clone();
        let refreshing = refreshing.clone();
        let shape_value = shape_value.clone();
        dropdown.connect_selected_notify(move |d| {
            sides.set_visible(d.selected() == custom_index);
            if refreshing.get() {
                return;
            }
            ctx.set(vec![(key, shape_value(d.selected()))]);
        });
    }
    {
        let dropdown = dropdown.clone();
        let refreshing = refreshing.clone();
        sides.connect_value_changed(move |sp| {
            if refreshing.get() || dropdown.selected() != custom_index {
                return;
            }
            ctx.set(vec![(key, json!({ "polygon": sp.value() as u32 }))]);
        });
    }
    let refresh: FieldRefresh = {
        let dropdown = dropdown.clone();
        let sides = sides.clone();
        Box::new(move |m: &Map<String, Value>| {
            let shape = m
                .get(key)
                .and_then(|v| serde_json::from_value::<IndicatorShape>(v.clone()).ok())
                .unwrap_or_default();
            let index = match shape {
                IndicatorShape::Fill => 0,
                IndicatorShape::Circle => 1,
                IndicatorShape::Square => 2,
                IndicatorShape::Polygon(n) => PRESETS
                    .iter()
                    .position(|(_, p)| *p == Some(n))
                    .map(|i| i as u32)
                    .unwrap_or(custom_index),
            };
            refreshing.set(true);
            if let IndicatorShape::Polygon(n) = shape {
                sides.set_value(n as f64);
            }
            dropdown.set_selected(index);
            sides.set_visible(index == custom_index);
            refreshing.set(false);
        })
    };
    FieldEditor::new(inner, refresh)
}
