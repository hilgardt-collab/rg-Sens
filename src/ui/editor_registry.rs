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
use crate::ui::combo_config_base::{self, TransferableComboConfig};
use crate::ui::theme::ComboThemeConfig;
use crate::ui::ComboReorder;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Notebook, Widget};
use rg_sens_types::combo::{ComboFrameConfig, LayoutFrameConfig, ThemedFrameConfig};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Change notification handed to an editor.
pub type ChangeCallback = Rc<dyn Fn()>;

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
    /// The combo-specific surface, for the ten themed displayers.
    fn combo(&self) -> Option<&dyn ComboDisplayEditor> {
        None
    }
}

/// Extra surface of the themed combo displayer editors.
pub trait ComboDisplayEditor {
    /// Detach the editor's notebook pages as `(tab label, page)`. Call once;
    /// the editor's own container is not shown afterwards. The "Content"
    /// page is returned too; the caller strips its per-slot notebook.
    fn take_pages(&self) -> Vec<(String, Widget)>;
    /// Write the frame's per-group size weights directly (live drags of the
    /// group node's weight spin); the source stays the owner and the next
    /// apply re-derives them.
    fn set_group_weights(&self, weights: &[f64]);
    fn transferable(&self) -> TransferableComboConfig;
    fn apply_transferable(&self, transfer: &TransferableComboConfig);
    /// Apply a group/item swap to the frame config (content item keys move
    /// with their sources).
    fn reorder(&self, op: ComboReorder);
    /// Build the per-item display editor (display-as dropdown plus the
    /// embedded, lazily built display editors) bound to this editor's config.
    fn item_editor(
        &self,
        slot: &str,
        fields: &[FieldMetadata],
        on_change: ChangeCallback,
        theme_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
        cleanup: &Rc<RefCell<Vec<Box<dyn Fn()>>>>,
    ) -> Widget;
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
    // set_config takes the config by value
    ($name:ident, $widget:ident, $variant:ident, by_value, $setup:expr) => {
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
                    self.0.set_config(c.clone());
                }
            }
            fn store(&self) -> Option<SourceConfig> {
                Some(SourceConfig::$variant(self.0.get_config()))
            }
        }
    };
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

source_editor!(CpuEditor, CpuSourceConfigWidget, Cpu, by_value, |w| {
    w.set_available_sensors(crate::sources::CpuSource::get_cached_sensors());
    w.set_cpu_core_count(crate::sources::CpuSource::get_cached_core_count());
});
source_editor!(GpuEditor, GpuSourceConfigWidget, Gpu, by_value, |w| {
    let names = crate::sources::GpuSource::get_cached_gpu_names();
    w.set_available_gpus(&names);
});
source_editor!(
    MemoryEditor,
    MemorySourceConfigWidget,
    Memory,
    by_value,
    |_w| {}
);
source_editor!(
    SystemTempEditor,
    SystemTempConfigWidget,
    SystemTemp,
    by_value,
    |_w| {}
);
source_editor!(
    FanSpeedEditor,
    FanSpeedConfigWidget,
    FanSpeed,
    by_ref,
    |_w| {}
);
source_editor!(DiskEditor, DiskSourceConfigWidget, Disk, by_value, |w| {
    w.set_available_disks(&crate::sources::DiskSource::get_available_disks());
});
source_editor!(
    NetworkEditor,
    NetworkSourceConfigWidget,
    Network,
    by_value,
    |w| {
        w.set_available_interfaces(&crate::sources::NetworkSource::get_available_interfaces());
    }
);
source_editor!(ClockEditor, ClockSourceConfigWidget, Clock, by_ref, |_w| {});
source_editor!(
    ClaudeEditor,
    ClaudeSourceConfigWidget,
    Claude,
    by_value,
    |_w| {}
);
source_editor!(
    StaticTextEditor,
    StaticTextConfigWidget,
    StaticText,
    by_ref,
    |_w| {}
);
source_editor!(TestEditor, TestSourceConfigWidget, Test, by_ref, |_w| {});

// ============================================================================
// Standalone display editor adapters
// ============================================================================

pub struct TextEditor(crate::ui::TextLineConfigWidget);
impl DisplayEditor for TextEditor {
    fn widget(&self) -> Widget {
        self.0.widget().clone()
    }
    fn load(&self, config: &DisplayerConfig) {
        if let DisplayerConfig::Text(c) = config {
            self.0.set_config(c.clone());
        }
    }
    fn store(&self) -> Option<DisplayerConfig> {
        Some(DisplayerConfig::Text(self.0.get_config()))
    }
    fn set_on_change(&self, cb: ChangeCallback) -> bool {
        self.0.set_on_change(move || cb());
        true
    }
    fn set_theme(&self, theme: &ComboThemeConfig) {
        self.0.set_theme(theme.clone());
    }
    fn cleanup(&self) {
        self.0.cleanup();
    }
}

pub struct BarEditor(crate::ui::BarConfigWidget);
impl DisplayEditor for BarEditor {
    fn widget(&self) -> Widget {
        self.0.widget().clone().upcast()
    }
    fn load(&self, config: &DisplayerConfig) {
        if let DisplayerConfig::Bar(c) = config {
            self.0.set_config(c.clone());
        }
    }
    fn store(&self) -> Option<DisplayerConfig> {
        Some(DisplayerConfig::Bar(self.0.get_config()))
    }
    fn set_on_change(&self, cb: ChangeCallback) -> bool {
        self.0.set_on_change(move || cb());
        true
    }
    fn set_theme(&self, theme: &ComboThemeConfig) {
        self.0.set_theme(theme.clone());
    }
    fn cleanup(&self) {
        self.0.cleanup();
    }
}

pub struct ArcEditor(crate::ui::ArcConfigWidget);
impl DisplayEditor for ArcEditor {
    fn widget(&self) -> Widget {
        self.0.widget().clone().upcast()
    }
    fn load(&self, config: &DisplayerConfig) {
        if let DisplayerConfig::Arc(c) = config {
            self.0.set_config(c.clone());
        }
    }
    fn store(&self) -> Option<DisplayerConfig> {
        Some(DisplayerConfig::Arc(self.0.get_config()))
    }
    fn set_on_change(&self, cb: ChangeCallback) -> bool {
        self.0.set_on_change(move || cb());
        true
    }
    fn set_theme(&self, theme: &ComboThemeConfig) {
        self.0.set_theme(theme.clone());
    }
    fn cleanup(&self) {
        self.0.cleanup();
    }
}

pub struct SpeedometerEditor(crate::ui::SpeedometerConfigWidget);
impl DisplayEditor for SpeedometerEditor {
    fn widget(&self) -> Widget {
        self.0.widget().clone().upcast()
    }
    fn load(&self, config: &DisplayerConfig) {
        if let DisplayerConfig::Speedometer(c) = config {
            self.0.set_config(c);
        }
    }
    fn store(&self) -> Option<DisplayerConfig> {
        Some(DisplayerConfig::Speedometer(self.0.get_config()))
    }
    fn set_on_change(&self, cb: ChangeCallback) -> bool {
        self.0.set_on_change(move || cb());
        true
    }
    fn set_theme(&self, theme: &ComboThemeConfig) {
        self.0.set_theme(theme.clone());
    }
    fn cleanup(&self) {
        self.0.cleanup();
    }
}

pub struct GraphEditor(crate::ui::GraphConfigWidget);
impl DisplayEditor for GraphEditor {
    fn widget(&self) -> Widget {
        self.0.widget().clone().upcast()
    }
    fn load(&self, config: &DisplayerConfig) {
        if let DisplayerConfig::Graph(c) = config {
            self.0.set_config(c.clone());
        }
    }
    fn store(&self) -> Option<DisplayerConfig> {
        Some(DisplayerConfig::Graph(self.0.get_config()))
    }
    fn set_on_change(&self, cb: ChangeCallback) -> bool {
        self.0.set_on_change(move || cb());
        true
    }
    fn set_theme(&self, theme: &ComboThemeConfig) {
        self.0.set_theme(theme.clone());
    }
    fn cleanup(&self) {
        self.0.cleanup();
    }
}

pub struct ClockAnalogEditor(crate::ui::ClockAnalogConfigWidget);
impl DisplayEditor for ClockAnalogEditor {
    fn widget(&self) -> Widget {
        self.0.widget().clone().upcast()
    }
    fn load(&self, config: &DisplayerConfig) {
        if let DisplayerConfig::ClockAnalog(c) = config {
            self.0.set_config(c.clone());
        }
    }
    fn store(&self) -> Option<DisplayerConfig> {
        Some(DisplayerConfig::ClockAnalog(self.0.get_config()))
    }
    fn set_theme(&self, theme: &ComboThemeConfig) {
        self.0.set_theme(theme.clone());
    }
    fn cleanup(&self) {
        self.0.cleanup();
    }
}

pub struct ClockDigitalEditor(crate::ui::ClockDigitalConfigWidget);
impl DisplayEditor for ClockDigitalEditor {
    fn widget(&self) -> Widget {
        self.0.widget().clone().upcast()
    }
    fn load(&self, config: &DisplayerConfig) {
        if let DisplayerConfig::ClockDigital(c) = config {
            self.0.set_config(c.clone());
        }
    }
    fn store(&self) -> Option<DisplayerConfig> {
        Some(DisplayerConfig::ClockDigital(self.0.get_config()))
    }
}

pub struct CpuCoresEditor(crate::ui::CoreBarsConfigWidget);
impl DisplayEditor for CpuCoresEditor {
    fn widget(&self) -> Widget {
        self.0.widget().clone().upcast()
    }
    fn load(&self, config: &DisplayerConfig) {
        if let DisplayerConfig::CpuCores(c) = config {
            self.0.set_config(c.clone());
        }
    }
    fn store(&self) -> Option<DisplayerConfig> {
        Some(DisplayerConfig::CpuCores(self.0.get_config()))
    }
    fn set_on_change(&self, cb: ChangeCallback) -> bool {
        self.0.set_on_change(move || cb());
        true
    }
    fn set_theme(&self, theme: &ComboThemeConfig) {
        self.0.set_theme(theme.clone());
    }
    fn cleanup(&self) {
        self.0.cleanup();
    }
}

pub struct IndicatorEditor(crate::ui::IndicatorConfigWidget);
impl DisplayEditor for IndicatorEditor {
    fn widget(&self) -> Widget {
        self.0.widget().clone().upcast()
    }
    fn load(&self, config: &DisplayerConfig) {
        if let DisplayerConfig::Indicator(c) = config {
            self.0.set_config(c);
        }
    }
    fn store(&self) -> Option<DisplayerConfig> {
        Some(DisplayerConfig::Indicator(self.0.get_config()))
    }
    fn set_on_change(&self, cb: ChangeCallback) -> bool {
        self.0.set_on_change(move || cb());
        true
    }
}

#[cfg(feature = "webkit")]
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

/// Detach every page of the first `Notebook` found among `container`'s
/// children, returning `(tab label, page)` in tab order.
fn detach_notebook_pages(container: &GtkBox) -> Vec<(String, Widget)> {
    let mut pages = Vec::new();
    let mut child = container.first_child();
    while let Some(c) = child {
        child = c.next_sibling();
        let Ok(notebook) = c.downcast::<Notebook>() else {
            continue;
        };
        let n = notebook.n_pages();
        for i in 0..n {
            if let Some(page) = notebook.nth_page(Some(i)) {
                let label = notebook
                    .tab_label_text(&page)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("Page {}", i + 1));
                pages.push((label, page));
            }
        }
        for (_, page) in &pages {
            if let Some(idx) = notebook.page_num(page) {
                notebook.remove_page(Some(idx));
            }
        }
        break;
    }
    pages
}

macro_rules! combo_editor {
    ($name:ident, $widget:ident, $variant:ident, $cfg:ty, set_by_ref) => {
        combo_editor!(@impl $name, $widget, $variant, $cfg, |w: &crate::ui::$widget, c: &$cfg| w.set_config(c));
    };
    ($name:ident, $widget:ident, $variant:ident, $cfg:ty, set_by_value) => {
        combo_editor!(@impl $name, $widget, $variant, $cfg, |w: &crate::ui::$widget, c: &$cfg| w.set_config(c.clone()));
    };
    (@impl $name:ident, $widget:ident, $variant:ident, $cfg:ty, $set:expr) => {
        pub struct $name(crate::ui::$widget);
        impl DisplayEditor for $name {
            fn widget(&self) -> Widget {
                self.0.widget().clone().upcast()
            }
            fn load(&self, config: &DisplayerConfig) {
                if let DisplayerConfig::$variant(c) = config {
                    let set: fn(&crate::ui::$widget, &$cfg) = $set;
                    set(&self.0, c);
                }
            }
            fn store(&self) -> Option<DisplayerConfig> {
                Some(DisplayerConfig::$variant(self.0.get_config()))
            }
            fn set_on_change(&self, cb: ChangeCallback) -> bool {
                self.0.set_on_change(move || cb());
                true
            }
            fn set_theme(&self, theme: &ComboThemeConfig) {
                self.0.set_theme(theme.clone());
            }
            fn set_source_summaries(&self, summaries: Vec<(String, String, usize, u32)>) {
                self.0.set_source_summaries(summaries);
            }
            fn cleanup(&self) {
                self.0.cleanup();
            }
            fn combo(&self) -> Option<&dyn ComboDisplayEditor> {
                Some(self)
            }
        }
        impl ComboDisplayEditor for $name {
            fn take_pages(&self) -> Vec<(String, Widget)> {
                detach_notebook_pages(self.0.widget())
            }
            fn set_group_weights(&self, weights: &[f64]) {
                let rc = self.0.get_config_rc();
                *rc.borrow_mut().frame.group_size_weights_mut() = weights.to_vec();
            }
            fn transferable(&self) -> TransferableComboConfig {
                self.0.get_transferable_config()
            }
            fn apply_transferable(&self, transfer: &TransferableComboConfig) {
                self.0.apply_transferable_config(transfer);
            }
            fn reorder(&self, op: ComboReorder) {
                let rc = self.0.get_config_rc();
                let mut cfg = rc.borrow_mut();
                match op {
                    ComboReorder::Group { a, b } => cfg.frame.swap_groups(a, b),
                    ComboReorder::Item { group, i, j } => cfg.frame.swap_items(group, i, j),
                }
            }
            fn item_editor(
                &self,
                slot: &str,
                fields: &[FieldMetadata],
                on_change: ChangeCallback,
                theme_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
                cleanup: &Rc<RefCell<Vec<Box<dyn Fn()>>>>,
            ) -> Widget {
                let rc = self.0.get_config_rc();
                let on_change_cell: Rc<RefCell<Option<Box<dyn Fn()>>>> =
                    Rc::new(RefCell::new(Some(Box::new(move || on_change()))));
                // The item editors queue a redraw on a preview; the live panel
                // is the preview here, so give them an unparented one.
                let preview = gtk4::DrawingArea::new();
                combo_config_base::create_content_item_config(
                    &rc,
                    &on_change_cell,
                    &preview,
                    slot,
                    fields.to_vec(),
                    |c: &$cfg| c.frame.content_items(),
                    |c: &mut $cfg, s: &str, item| {
                        c.frame.content_items_mut().insert(s.to_string(), item);
                    },
                    theme_refreshers,
                    |c: &$cfg| c.frame.theme().clone(),
                    cleanup,
                )
                .upcast()
            }
            fn refresh_theme_refs(&self) {
                // set_theme on the widget already runs its refreshers.
            }
        }
    };
}

use rg_sens_types::display_configs::themed_configs::{
    ArtDecoDisplayConfig, ArtNouveauDisplayConfig, CyberpunkDisplayConfig, FighterHudDisplayConfig,
    IndustrialDisplayConfig, LcarsDisplayConfig, MaterialDisplayConfig, RetroTerminalDisplayConfig,
    SteampunkDisplayConfig, SynthwaveDisplayConfig,
};

combo_editor!(
    LcarsEditor,
    LcarsConfigWidget,
    Lcars,
    LcarsDisplayConfig,
    set_by_value
);
combo_editor!(
    CyberpunkEditor,
    CyberpunkConfigWidget,
    Cyberpunk,
    CyberpunkDisplayConfig,
    set_by_ref
);
combo_editor!(
    MaterialEditor,
    MaterialConfigWidget,
    Material,
    MaterialDisplayConfig,
    set_by_ref
);
combo_editor!(
    IndustrialEditor,
    IndustrialConfigWidget,
    Industrial,
    IndustrialDisplayConfig,
    set_by_ref
);
combo_editor!(
    RetroTerminalEditor,
    RetroTerminalConfigWidget,
    RetroTerminal,
    RetroTerminalDisplayConfig,
    set_by_ref
);
combo_editor!(
    FighterHudEditor,
    FighterHudConfigWidget,
    FighterHud,
    FighterHudDisplayConfig,
    set_by_value
);
combo_editor!(
    SynthwaveEditor,
    SynthwaveConfigWidget,
    Synthwave,
    SynthwaveDisplayConfig,
    set_by_value
);
combo_editor!(
    ArtDecoEditor,
    ArtDecoConfigWidget,
    ArtDeco,
    ArtDecoDisplayConfig,
    set_by_ref
);
combo_editor!(
    ArtNouveauEditor,
    ArtNouveauConfigWidget,
    ArtNouveau,
    ArtNouveauDisplayConfig,
    set_by_ref
);
combo_editor!(
    SteampunkEditor,
    SteampunkConfigWidget,
    Steampunk,
    SteampunkDisplayConfig,
    set_by_ref
);

// ============================================================================
// Registration
// ============================================================================

/// Register every built-in editor. Called once at startup next to the
/// source/displayer registration.
pub fn register_all_editors() {
    register_source_editor("cpu", CpuEditor::create);
    register_source_editor("gpu", GpuEditor::create);
    register_source_editor("memory", MemoryEditor::create);
    register_source_editor("system_temp", SystemTempEditor::create);
    register_source_editor("fan_speed", FanSpeedEditor::create);
    register_source_editor("disk", DiskEditor::create);
    register_source_editor("network", NetworkEditor::create);
    register_source_editor("clock", ClockEditor::create);
    register_source_editor("claude", ClaudeEditor::create);
    register_source_editor("static_text", StaticTextEditor::create);
    register_source_editor("test", TestEditor::create);

    register_display_editor("text", |fields| {
        Box::new(TextEditor(crate::ui::TextLineConfigWidget::new(fields)))
    });
    register_display_editor("bar", |fields| {
        Box::new(BarEditor(crate::ui::BarConfigWidget::new(fields)))
    });
    register_display_editor("arc", |fields| {
        Box::new(ArcEditor(crate::ui::ArcConfigWidget::new(fields)))
    });
    register_display_editor("speedometer", |fields| {
        Box::new(SpeedometerEditor(crate::ui::SpeedometerConfigWidget::new(
            fields,
        )))
    });
    register_display_editor("graph", |fields| {
        Box::new(GraphEditor(crate::ui::GraphConfigWidget::new(fields)))
    });
    register_display_editor("clock_analog", |_| {
        Box::new(ClockAnalogEditor(crate::ui::ClockAnalogConfigWidget::new()))
    });
    register_display_editor("clock_digital", |_| {
        Box::new(ClockDigitalEditor(
            crate::ui::ClockDigitalConfigWidget::new(),
        ))
    });
    register_display_editor("cpu_cores", |fields| {
        let w = crate::ui::CoreBarsConfigWidget::with_fields(&fields);
        w.set_max_cores(crate::sources::CpuSource::get_cached_core_count());
        Box::new(CpuCoresEditor(w))
    });
    register_display_editor("indicator", |fields| {
        Box::new(IndicatorEditor(crate::ui::IndicatorConfigWidget::new(
            fields,
        )))
    });
    #[cfg(feature = "webkit")]
    register_display_editor("css_template", |_| {
        let w = crate::ui::CssTemplateConfigWidget::new();
        w.connect_auto_config_handler();
        w.connect_scan_handler();
        Box::new(CssTemplateEditor(w))
    });

    register_display_editor("lcars", |f| {
        Box::new(LcarsEditor(crate::ui::LcarsConfigWidget::new(f)))
    });
    register_display_editor("cyberpunk", |f| {
        Box::new(CyberpunkEditor(crate::ui::CyberpunkConfigWidget::new(f)))
    });
    register_display_editor("material", |f| {
        Box::new(MaterialEditor(crate::ui::MaterialConfigWidget::new(f)))
    });
    register_display_editor("industrial", |f| {
        Box::new(IndustrialEditor(crate::ui::IndustrialConfigWidget::new(f)))
    });
    register_display_editor("retro_terminal", |f| {
        Box::new(RetroTerminalEditor(
            crate::ui::RetroTerminalConfigWidget::new(f),
        ))
    });
    register_display_editor("fighter_hud", |f| {
        Box::new(FighterHudEditor(crate::ui::FighterHudConfigWidget::new(f)))
    });
    register_display_editor("synthwave", |f| {
        Box::new(SynthwaveEditor(crate::ui::SynthwaveConfigWidget::new(f)))
    });
    register_display_editor("art_deco", |f| {
        Box::new(ArtDecoEditor(crate::ui::ArtDecoConfigWidget::new(f)))
    });
    register_display_editor("art_nouveau", |f| {
        Box::new(ArtNouveauEditor(crate::ui::ArtNouveauConfigWidget::new(f)))
    });
    register_display_editor("steampunk", |f| {
        Box::new(SteampunkEditor(crate::ui::SteampunkConfigWidget::new(f)))
    });
}
