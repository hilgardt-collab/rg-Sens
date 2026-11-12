//! Editor registry: the GTK configuration editor for every source and
//! displayer, keyed by the same ids the core `Registry` uses.
//!
//! This is the one place that knows which `*_config_widget.rs` edits which
//! config. The tree-based properties dialog (`config_tree`) asks for an
//! editor by id and talks to it only through [`SourceEditor`] /
//! [`DisplayEditor`], so adding a source or displayer means one factory
//! registration here, not another arm in a dialog `match`.
//!
//! Adapters are thin: they wrap the existing widget and translate between
//! its concrete config type and the typed `SourceConfig` /
//! `DisplayerConfig` enums.

use crate::core::{DisplayerConfig, FieldMetadata, SourceConfig};
use crate::ui::config_tree::combo_items::ItemDisplayEditor;
use crate::ui::config_tree::field_editors::register_field_editors;
use crate::ui::config_tree::schema_combo::SchemaComboEditor;
use crate::ui::config_tree::schema_editor::{
    register_choice_provider, SchemaDisplayEditor, SchemaSourceEditor,
};
use crate::ui::theme::ComboThemeConfig;
use gtk4::prelude::*;
use gtk4::Widget;
use rg_sens_types::combo::TransferableComboConfig;
use rg_sens_types::display_configs::synthwave::SynthwaveFrameConfig;
use rg_sens_types::display_configs::{
    AnalogClockConfig, ArcDisplayConfig, BarDisplayConfig, CoreBarsConfig, DigitalClockConfig,
    GraphDisplayConfig, IndicatorConfig, SpeedometerConfig,
};
use rg_sens_types::display_configs::{
    ArtDecoFrameConfig, ArtNouveauFrameConfig, CyberpunkFrameConfig, FighterHudFrameConfig,
    IndustrialFrameConfig, LcarsFrameConfig, MaterialFrameConfig, RetroTerminalFrameConfig,
    SteampunkFrameConfig,
};
use rg_sens_types::schema::Schema;
use rg_sens_types::schema::TreePage;
use rg_sens_types::source_configs::claude::ClaudeSourceConfig;
use rg_sens_types::source_configs::clock::ClockSourceConfig;
use rg_sens_types::source_configs::cpu::CpuSourceConfig;
use rg_sens_types::source_configs::fan_speed::FanSpeedConfig;
use rg_sens_types::source_configs::gpu::GpuSourceConfig;
use rg_sens_types::source_configs::static_text::StaticTextSourceConfig;
use rg_sens_types::source_configs::uptime::UptimeSourceConfig;
use rg_sens_types::source_configs::system_temp::SystemTempConfig;
use rg_sens_types::source_configs::{DiskSourceConfig, MemorySourceConfig, NetworkSourceConfig};
use rg_sens_types::text::TextDisplayerConfig;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A group or item swap requested from the properties tree. The dialog
/// applies it to the combo source config and, through
/// [`ComboDisplayEditor::reorder`], to the display config, so per-slot
/// display settings stay attached to their source. All indices are 0-based.
#[derive(Clone, Copy, Debug)]
pub enum ComboReorder {
    /// Swap two groups.
    Group { a: usize, b: usize },
    /// Swap two items within a group.
    Item { group: usize, i: usize, j: usize },
}

/// Change notification handed to an editor.
pub type ChangeCallback = Rc<dyn Fn()>;

/// One page of a display editor, addressed by path (see
/// [`DisplayEditor::sections`]).
#[derive(Clone)]
pub struct EditorPage {
    pub path: Vec<String>,
    pub widget: Widget,
}

/// Editor for one source type's configuration.
pub trait SourceEditor {
    fn widget(&self) -> Widget;
    /// Load a config; ignored if it is for another source type.
    fn load(&self, config: &SourceConfig);
    fn store(&self) -> Option<SourceConfig>;
    /// Returns `false` when the editor has no change signal, in which case
    /// the host polls `store()` while the editor is visible.
    fn set_on_change(&self, _cb: ChangeCallback) -> bool {
        false
    }
    fn cleanup(&self) {}
    /// Words the tree search box may match for this editor (schema labels).
    fn search_terms(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Editor for one displayer type's configuration.
pub trait DisplayEditor {
    fn widget(&self) -> Widget;
    fn load(&self, config: &DisplayerConfig);
    fn store(&self) -> Option<DisplayerConfig>;
    /// See [`SourceEditor::set_on_change`].
    fn set_on_change(&self, _cb: ChangeCallback) -> bool {
        false
    }
    fn set_theme(&self, _theme: &ComboThemeConfig) {}
    /// Combo-compatible displayers get the slot summaries (css_template
    /// uses them for its placeholder mapping).
    fn set_source_summaries(&self, _summaries: Vec<(String, String, usize, u32)>) {}
    fn cleanup(&self) {}
    /// An editor may split into pages; the tree shows them as child nodes
    /// of Display. A one-element path is a section; a longer path (a list
    /// item) is filed under its nearest ancestor page, or Display.
    fn sections(&self) -> Vec<EditorPage> {
        Vec::new()
    }
    /// Live title of a page (list item titles follow their content).
    fn page_title(&self, path: &[String]) -> String {
        path.last().cloned().unwrap_or_default()
    }
    /// Words the tree search box may match for one page.
    fn page_search_terms(&self, _path: &[String]) -> Vec<String> {
        Vec::new()
    }
    /// Called with the path to select (or `None` to keep the selection)
    /// whenever `sections()` would now return a different set of pages.
    fn set_on_sections_changed(&self, _cb: Rc<dyn Fn(Option<Vec<String>>)>) {}
    /// The combo-specific surface, for the ten themed displayers.
    fn combo(&self) -> Option<&dyn ComboDisplayEditor> {
        None
    }
}

/// Extra surface of the themed combo displayer editors.
pub trait ComboDisplayEditor {
    /// Write the frame's per-group size weights directly (live drags of the
    /// group node's weight spin); the source stays the owner and the next
    /// apply re-derives them.
    fn set_group_weights(&self, weights: &[f64]);
    fn transferable(&self) -> TransferableComboConfig;
    fn apply_transferable(&self, transfer: &TransferableComboConfig);
    /// Apply a group/item swap to the frame config (content item keys move
    /// with their sources).
    fn reorder(&self, op: ComboReorder);
    /// Build the per-item display editor (display-as dropdown, height, and
    /// the display type's schema editor, whose pages the tree shows as child
    /// nodes of the item) bound to this editor's config.
    fn item_editor(
        &self,
        slot: &str,
        source_id: &str,
        fields: &[FieldMetadata],
        on_change: ChangeCallback,
    ) -> Rc<ItemDisplayEditor>;
    /// Make the slot's item use a display type its source offers (a clock is
    /// shown as text or as an analog face, a metric never as a face); returns
    /// whether the frame changed. Silent: the caller commits.
    fn coerce_item_display(&self, slot: &str, source_id: &str) -> bool;
    /// The pages an item's display editor has (or would have), derived from
    /// the config alone, so the tree lists every item's child nodes without
    /// building the editors.
    fn item_pages(&self, slot: &str) -> Vec<TreePage>;
    /// The slot's `ContentItemConfig` as JSON, if the frame has one.
    fn content_item(&self, slot: &str) -> Option<Value>;
    /// Replace the slot's `ContentItemConfig` (silent: the caller rebuilds
    /// the item's editors and commits).
    fn set_content_item(&self, slot: &str, item: Value);
    /// The group's title bar (`GroupTitleConfig` as JSON; the default when
    /// the frame has none for it).
    fn group_title(&self, group: usize) -> Value;
    /// Write the group's title bar, padding the frame's list with defaults
    /// up to the group (silent: the caller commits).
    fn set_group_title(&self, group: usize, title: Value);
    /// How the group lays out its items (`"vertical"` / `"horizontal"`): its
    /// own entry, else the frame's group layout.
    fn group_orientation(&self, group: usize) -> String;
    /// Write the group's item orientation, padding the frame's list with the
    /// group layout up to the group (silent: the caller commits).
    fn set_group_orientation(&self, group: usize, orientation: &str);
    /// Trigger the editor's own theme-reference refreshers after
    /// `set_theme` (page colour pickers show the resolved theme).
    fn refresh_theme_refs(&self);
}

type SourceFactory = Rc<dyn Fn() -> Box<dyn SourceEditor>>;
type DisplayFactory = Rc<dyn Fn(Vec<FieldMetadata>) -> Box<dyn DisplayEditor>>;

thread_local! {
    static SOURCE_EDITORS: RefCell<HashMap<String, SourceFactory>> = RefCell::new(HashMap::new());
    static DISPLAY_EDITORS: RefCell<HashMap<String, DisplayFactory>> = RefCell::new(HashMap::new());
}

pub fn register_source_editor<F>(id: &str, factory: F)
where
    F: Fn() -> Box<dyn SourceEditor> + 'static,
{
    SOURCE_EDITORS.with(|m| {
        m.borrow_mut().insert(id.to_string(), Rc::new(factory));
    });
}

pub fn register_display_editor<F>(id: &str, factory: F)
where
    F: Fn(Vec<FieldMetadata>) -> Box<dyn DisplayEditor> + 'static,
{
    DISPLAY_EDITORS.with(|m| {
        m.borrow_mut().insert(id.to_string(), Rc::new(factory));
    });
}

/// Create the editor for a source id, if one is registered.
pub fn create_source_editor(id: &str) -> Option<Box<dyn SourceEditor>> {
    let factory = SOURCE_EDITORS.with(|m| m.borrow().get(id).cloned())?;
    Some(factory())
}

/// Whether a source id has an editor (a source without one has no settings
/// to show).
pub fn has_source_editor(id: &str) -> bool {
    SOURCE_EDITORS.with(|m| m.borrow().contains_key(id))
}

/// Create the editor for a displayer id, if one is registered.
pub fn create_display_editor(
    id: &str,
    fields: Vec<FieldMetadata>,
) -> Option<Box<dyn DisplayEditor>> {
    let factory = DISPLAY_EDITORS.with(|m| m.borrow().get(id).cloned())?;
    Some(factory(fields))
}

/// Whether a displayer id has a themed combo editor.
pub fn is_combo_display(id: &str) -> bool {
    matches!(
        id,
        "lcars"
            | "cyberpunk"
            | "material"
            | "industrial"
            | "retro_terminal"
            | "fighter_hud"
            | "synthwave"
            | "art_deco"
            | "art_nouveau"
            | "steampunk"
    )
}

// ============================================================================
// Source editor adapters
// ============================================================================

macro_rules! source_editor {
    // set_config takes a reference
    ($name:ident, $widget:ident, $variant:ident, by_ref, $setup:expr) => {
        pub struct $name(crate::ui::$widget);
        impl $name {
            fn create() -> Box<dyn SourceEditor> {
                let w = crate::ui::$widget::new();
                let setup: fn(&crate::ui::$widget) = $setup;
                setup(&w);
                Box::new(Self(w))
            }
        }
        impl SourceEditor for $name {
            fn widget(&self) -> Widget {
                self.0.widget().clone().upcast()
            }
            fn load(&self, config: &SourceConfig) {
                if let SourceConfig::$variant(c) = config {
                    self.0.set_config(c);
                }
            }
            fn store(&self) -> Option<SourceConfig> {
                Some(SourceConfig::$variant(self.0.get_config()))
            }
        }
    };
}

source_editor!(TestEditor, TestSourceConfigWidget, Test, by_ref, |_w| {});

// ============================================================================
// Standalone display editor adapters
// ============================================================================

pub struct CssTemplateEditor(crate::ui::CssTemplateConfigWidget);
#[cfg(feature = "webkit")]
impl DisplayEditor for CssTemplateEditor {
    fn widget(&self) -> Widget {
        self.0.widget().clone().upcast()
    }
    fn load(&self, config: &DisplayerConfig) {
        if let DisplayerConfig::CssTemplate(c) = config {
            self.0.set_config(c);
        }
    }
    fn store(&self) -> Option<DisplayerConfig> {
        Some(DisplayerConfig::CssTemplate(self.0.get_config()))
    }
    fn set_on_change(&self, cb: ChangeCallback) -> bool {
        self.0.set_on_change(move || cb());
        true
    }
    fn set_source_summaries(&self, summaries: Vec<(String, String, usize, u32)>) {
        self.0.set_source_summaries(summaries);
    }
}

// ============================================================================
// Themed combo display editor adapters
// ============================================================================

// ============================================================================
// Registration
// ============================================================================

/// Register every built-in editor. Called once at startup next to the
/// source/displayer registration.
pub fn register_all_editors() {
    // Choice lists for schema-driven editors (read once, cached by the sources).
    register_choice_provider("network_interfaces", || {
        crate::sources::NetworkSource::get_available_interfaces()
            .into_iter()
            .map(|n| (n.clone(), n))
            .collect()
    });
    register_choice_provider("disk_mounts", || {
        crate::sources::DiskSource::get_available_disks()
            .into_iter()
            .map(|(mount, name)| {
                let label = if name.is_empty() {
                    mount.clone()
                } else {
                    format!("{} ({})", mount, name)
                };
                (mount, label)
            })
            .collect()
    });

    register_choice_provider("cpu_sensors", || {
        crate::sources::CpuSource::get_cached_sensors()
            .iter()
            .map(|s| (s.index.to_string(), s.label.clone()))
            .collect()
    });
    register_choice_provider("gpus", || {
        crate::sources::GpuSource::get_cached_gpu_names()
            .into_iter()
            .enumerate()
            .map(|(i, name)| (i.to_string(), name))
            .collect()
    });
    register_field_editors();

    // Schema-driven sources: no widget, rows come from `Schema::props()`.
    register_source_editor("cpu", || {
        SchemaSourceEditor::create("cpu", CpuSourceConfig::props())
    });
    register_source_editor("gpu", || {
        SchemaSourceEditor::create("gpu", GpuSourceConfig::props())
    });
    register_source_editor("memory", || {
        SchemaSourceEditor::create("memory", MemorySourceConfig::props())
    });
    register_source_editor("system_temp", || {
        SchemaSourceEditor::create("system_temp", SystemTempConfig::props())
    });
    register_source_editor("fan_speed", || {
        SchemaSourceEditor::create("fan_speed", FanSpeedConfig::props())
    });
    register_source_editor("disk", || {
        SchemaSourceEditor::create("disk", DiskSourceConfig::props())
    });
    register_source_editor("network", || {
        SchemaSourceEditor::create("network", NetworkSourceConfig::props())
    });
    register_source_editor("clock", || {
        SchemaSourceEditor::create("clock", ClockSourceConfig::props())
    });
    register_source_editor("claude", || {
        SchemaSourceEditor::create_with_footer(
            "claude",
            ClaudeSourceConfig::props(),
            &crate::ui::claude_account::account_widget(),
        )
    });
    register_source_editor("static_text", || {
        SchemaSourceEditor::create("static_text", StaticTextSourceConfig::props())
    });
    register_source_editor("uptime", || {
        SchemaSourceEditor::create("uptime", UptimeSourceConfig::props())
    });
    // The test source keeps its widget: its values live in the global Test
    // Source dialog, the widget only edits the interval.
    register_source_editor("test", TestEditor::create);

    register_display_editor("text", |fields| {
        SchemaDisplayEditor::create("text", TextDisplayerConfig::props(), fields)
    });
    // Schema-driven displayers: sections become child nodes of Display.
    register_display_editor("bar", |fields| {
        SchemaDisplayEditor::create("bar", BarDisplayConfig::props(), fields)
    });
    register_display_editor("arc", |fields| {
        SchemaDisplayEditor::create("arc", ArcDisplayConfig::props(), fields)
    });
    register_display_editor("speedometer", |fields| {
        SchemaDisplayEditor::create("speedometer", SpeedometerConfig::props(), fields)
    });
    register_display_editor("graph", |fields| {
        SchemaDisplayEditor::create("graph", GraphDisplayConfig::props(), fields)
    });
    register_display_editor("clock_analog", |fields| {
        SchemaDisplayEditor::create("clock_analog", AnalogClockConfig::props(), fields)
    });
    register_display_editor("clock_digital", |fields| {
        SchemaDisplayEditor::create("clock_digital", DigitalClockConfig::props(), fields)
    });
    register_display_editor("cpu_cores", |fields| {
        SchemaDisplayEditor::create("cpu_cores", CoreBarsConfig::props(), fields)
    });
    register_display_editor("indicator", |fields| {
        SchemaDisplayEditor::create("indicator", IndicatorConfig::props(), fields)
    });
    #[cfg(feature = "webkit")]
    register_display_editor("css_template", |_| {
        let w = crate::ui::CssTemplateConfigWidget::new();
        w.connect_auto_config_handler();
        w.connect_scan_handler();
        Box::new(CssTemplateEditor(w))
    });

    // Schema-driven combo frames: sections and item pages from the schema.
    register_display_editor("lcars", |fields| {
        SchemaComboEditor::create("lcars", LcarsFrameConfig::props(), fields)
    });
    register_display_editor("cyberpunk", |fields| {
        SchemaComboEditor::create("cyberpunk", CyberpunkFrameConfig::props(), fields)
    });
    register_display_editor("material", |fields| {
        SchemaComboEditor::create("material", MaterialFrameConfig::props(), fields)
    });
    register_display_editor("industrial", |fields| {
        SchemaComboEditor::create("industrial", IndustrialFrameConfig::props(), fields)
    });
    register_display_editor("retro_terminal", |fields| {
        SchemaComboEditor::create("retro_terminal", RetroTerminalFrameConfig::props(), fields)
    });
    register_display_editor("fighter_hud", |fields| {
        SchemaComboEditor::create("fighter_hud", FighterHudFrameConfig::props(), fields)
    });
    register_display_editor("synthwave", |fields| {
        SchemaComboEditor::create("synthwave", SynthwaveFrameConfig::props(), fields)
    });
    register_display_editor("steampunk", |fields| {
        SchemaComboEditor::create("steampunk", SteampunkFrameConfig::props(), fields)
    });
    register_display_editor("art_nouveau", |fields| {
        SchemaComboEditor::create("art_nouveau", ArtNouveauFrameConfig::props(), fields)
    });
    register_display_editor("art_deco", |fields| {
        SchemaComboEditor::create("art_deco", ArtDecoFrameConfig::props(), fields)
    });
}
