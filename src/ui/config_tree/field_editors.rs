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
use crate::ui::TimezoneDialog;
use gtk4::prelude::*;
use gtk4::{Button, DropDown, StringList};
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
