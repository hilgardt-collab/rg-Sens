# Config UI Redesign: Tree-Based Configuration

Status: design accepted 2026-11-05 (Option A, see §8 and §9).
**Phase 0 implemented 2026-11-05** (see §10). **Phase 1 implemented
2026-11-05** (see §11), pending user testing against the §6 checklist;
the classic dialog is still reachable. Phases 2–3 not started.

Goal: replace the tab-in-tab-in-tab configuration dialogs with a tree that
shows the structure of what is being configured, without losing a single
option. Three designs are offered at the end; they share one foundation.

---

## 1. What exists today

### 1.1 The data model (what the UI has to edit)

```
AppConfig  (~/.config/rg-sens/config.json, version 2)
├── window: WindowConfig          size/pos, background, default corner+border,
│                                 fullscreen/monitor, borderless, auto-scroll, renderer
├── grid: GridConfig              columns, rows, cell w/h, spacing
├── global_theme: ComboThemeConfig   4 colours + gradient + 2 fonts
├── timers[], alarms[], global_timer_sound
└── panels[]: PanelConfigV2
    ├── id, geometry {x,y,w,h}
    ├── appearance: PanelAppearance   background, corner_radius, border,
    │                                 scale, translate_x/y, z_index, ignore_collision
    ├── source_config: SourceConfig       tagged enum, 12 variants
    │     cpu | gpu | memory | disk | network | clock | claude | system_temp
    │     fan_speed | test | static_text | combination
    └── displayer_config: DisplayerConfig  tagged enum, 20 variants
          text | bar | arc | speedometer | graph | clock_analog | clock_digital
          cpu_cores | indicator | css_template
          lcars | cyberpunk | material | industrial | retro_terminal
          fighter_hud | synthwave | art_deco | art_nouveau | steampunk
```

A **combo panel** is `source = combination` plus one of the ten themed
displayers. Its structure is stored **twice**:

```
SourceConfig::Combo                       DisplayerConfig::Lcars (etc.)
  groups[]: {item_count, size_weight}       frame.group_count
  slots{"group2_3": SlotConfig}             frame.group_item_counts[]
     source_id ("cpu")                      frame.group_size_weights[]
     caption_override                       frame.group_item_orientations[]
     source_config {…cpu fields…}           frame.content_items{"group2_3": ContentItemConfig}
                                               display_as (Bar|LevelBar|Text|Graph|CoreBars|Static|Arc|Speedometer)
                                               auto_height, item_height
                                               bar_config, graph_config, core_bars_config,
                                               static_config, arc_config, speedometer_config   ← ALL six, always
                                            frame.theme: ComboThemeConfig  (inline copy)
                                            frame.<style-specific fields>  (sidebar, headers, segments, bezel…)
```

The two halves are keyed by the same `"group{N}_{M}"` string and kept in
step by `crates/types/src/combo.rs::reorder` on reorder, and by
`set_source_summaries()` / `on_fields_updated` at dialog time.

Other stores the UI edits:

| Store | File | Edited from |
|---|---|---|
| `defaults.json` (per-displayer "default style", general defaults) | `src/config/defaults.rs` | context menu "Set as Default Style"; Window Settings › Defaults |
| typed clipboard (gradient, text line, font, colour, bar/graph/arc/speedometer/text display, cpu/gpu source, background, panel style, generic source config) | `src/ui/clipboard.rs` | Copy/Paste buttons inside each editor; context menu Copy/Paste Style |
| `~/.config/rg-sens/themes/` | nothing | exists on disk, empty, unreferenced by code |

Theme references: every colour/font in a config is a `ColorSource` /
`FontSource`, either `Custom` or `Theme { index }`. `Theme` resolves against
**a** `ComboThemeConfig`: for combo panels that is `frame.theme`; for
standalone panels it is `AppConfig.global_theme`, pushed into every
displayer under the `global_theme` key at startup (`src/main.rs:648`) and
whenever Window Settings saves it (`src/ui/window_settings_dialog.rs:974`),
consumed by `src/displayers/mod.rs:78 apply_global_theme`. There is no third
option.

Config size: `ContentItemConfig` serialises all six sub-configs whatever
`display_as` says (`crates/types/src/display_configs/lcars.rs:523`), about
17 KB per item. The author's real config is 4.2 MB for three panels of
~29 items each.

### 1.2 The UI surface

**Panel Properties** (`src/ui/grid_properties_dialog.rs`, 5 197 lines, one
function) is a five-tab `Notebook`:

```
Size | Data Source | Display Type | Background | Appearance
```

* Data Source: source dropdown + a lazily built config widget per source
  (11 placeholder blocks, `grid_properties_dialog.rs:296-480`).
* Display Type: displayer dropdown + a lazily built config widget per
  displayer (20 placeholder blocks, `:854-3195`).
* Apply handler: `:3453-5069`, ~1 600 lines, including 11-arm `match`es that
  must be extended for every new combo displayer (CLAUDE.md gotcha 15).

For a combo panel the path to one bar's gradient is:

```
Panel Properties
└ Display Type tab
  └ LCARS widget   [Theme|Frame|Headers|Segments|Content|Layout|Animation]
    └ Content tab
      └ Group 2 tab
        └ Item 2 tab
          └ Display as: Bar  +  embedded BarConfigWidget
                               [Bar Style|Foreground|Background|Style Options|…|Border|Animation]
            └ Foreground tab
              └ gradient editor
```

Five nested notebooks (Display Type › Content › Group › Item › Foreground),
nine clicks from the context menu. And the *source* for that same item is reached
from a different top-level tab (`Data Source › Combo › Group 2 › Item 2`),
which is why the Content tab carries the note "Configure data sources in the
'Data Source' tab first" (`src/ui/lcars_config_widget.rs:1515`).

**Window Settings** (`src/ui/window_settings_dialog.rs`, 1 040 lines):
`Defaults | Background | Window Mode | Global Theme`.

**New Panel** (`src/ui/new_panel_dialog.rs`): position, size, source,
displayer; then the user opens Properties to configure anything.

**Context menu per panel** (`src/ui/grid_layout.rs:75`): Properties…,
Copy Style, Paste Style, Set as Default Style, Save Panel to File…, Delete.

**Leaf editors**: 34 `src/ui/*_config_widget.rs` files, 34 793 lines, each
a self-contained widget with `set_config/get_config/set_on_change/set_theme/
cleanup` (`src/ui/config/config_widget_trait.rs`). These are the parts worth
keeping.

### 1.3 Concrete usability failures

1. **Split brain for combo items.** One item's data lives under
   `Data Source`, its look under `Display Type`, in two parallel
   Group/Item tab hierarchies the user must align by number.
2. **Depth.** Five nested notebooks to reach a colour. No breadcrumb; the
   only context is the currently visible tab labels.
3. **No overview.** Nothing shows which of 29 items are configured, which are
   `none`, which source feeds which display, or which panels share a theme.
4. **Structure owned twice.** Group/item counts live on both source and
   frame and are reconciled by widget callbacks, not by the model.
5. **Themes are either global or baked in.** A standalone bar panel can only
   follow the global theme; a combo embeds a private copy. There is no way to
   say "these six panels share theme X". The `themes/` directory exists for a
   feature that was never built.
6. **Hidden features.** Default styles, style copy/paste, per-editor
   copy/paste buttons and save-to-file are all present but discoverable only
   by accident, and behave differently (Paste Style writes to the live panel
   immediately and Cancel does not revert; `grid_properties_dialog.rs:3281`).
7. **Every new source or displayer touches six-plus places** in the dialog
   (placeholder, apply arm, `on_fields_updated` arm, `set_source_summaries`
   arm, `apply_reorder` arm, transferable-config arm).
8. **Multi-select exists for move/delete/copy, not for editing.**

---

## 2. Shared foundation (all three options build on this)

### 2.1 The node model

One Rust enum describes every thing that can be selected and edited. The tree
widget, the breadcrumb, the editor factory, and copy/paste all key off it.

```rust
pub enum NodeKind {
    // dashboard-level (Option B only shows these)
    Dashboard, WindowGrid, Themes, Theme(String), Defaults, TimersAlarms,
    Panels, Group(String),
    // per panel
    Panel(String),
    Geometry,                       // size, scale/offset, z-index, collision
    Source,                         // standalone: the source + its editor
    Display,                        // standalone: the displayer + its editor
    DisplaySection(&'static str),   // combo: "Theme" "Frame" "Headers" "Segments" "Layout" "Animation"
    Content,                        // combo: item spacing + group list
    ContentGroup(usize),            // combo: item_count, size_weight, orientation, reorder
    ContentItem(String),            // combo: "group2_3" — source AND display together
    Background,
    Appearance,                     // corner radius, border, copy/paste/default style
}
```

Nodes are `glib::Object` subclasses in a `gio::ListStore` per level, wrapped
in `gtk4::TreeListModel` and shown with `gtk4::ListView` +
`gtk4::TreeExpander`. (`TreeView` is deprecated at GTK 4.10 and CLAUDE.md
forbids deprecated widgets. libadwaita is not a dependency and this design
does not add it; `Paned` + `ListView` + `Stack` are enough.)

Each row shows: icon, label, and a dim summary (`CPU Temp → Bar`,
`3 items`, `theme: LCARS Classic`). Rows for empty slots (`source = none`)
render dimmed so gaps are visible at a glance.

### 2.2 The editor factory (kills the 11-arm matches)

```rust
pub trait NodeEditor {
    fn widget(&self) -> &gtk4::Widget;
    fn load(&self, ctx: &EditContext);          // typed config in
    fn store(&self, ctx: &mut EditContext);     // typed config out
    fn set_fields(&self, fields: &[FieldMetadata]) {}
    fn set_theme(&self, theme: &ComboThemeConfig) {}
    fn cleanup(&self) {}
}
```

Every existing `*_config_widget.rs` gets a thin `NodeEditor` adapter. The
`Registry` (`crates/core/src/registry.rs`) already knows every source and
displayer by id; it grows two maps:

```rust
registry.register_source_editor("cpu",  |fields| Box::new(CpuSourceEditor::new(fields)));
registry.register_displayer_editor("bar", |fields| Box::new(BarEditor::new(fields)));
```

Adding a source then means: source + config type + editor + one registration
line. The tree builder iterates `ComboSourceConfig.groups`/`slots` once and
asks the registry for editors; nothing in the dialog is per-displayer any more.

**Laziness is mandatory.** The git log for these dialogs is a trail of
memory and freeze fixes; the author's panels have 29 items each. An editor is
built on first selection of its node, kept alive for the dialog's lifetime,
and `cleanup()`ed on close. Combined with the existing async field cache
(`ComboSourceConfigWidget::update_fields_cache_async`) that keeps opening a
29-item panel as cheap as today's lazy placeholders.

### 2.3 Field flow (source → display) stays, but is local to the item

When a `ContentItem`'s source changes, the item node recomputes that slot's
`FieldMetadata` (same `CachedSources` mechanism) and pushes it to:
the item's display editor, the panel's Background editor (indicator
backgrounds pick a field), and the CSS-template mapping editor. No panel-wide
`on_fields_updated` fan-out.

### 2.4 One owner for combo structure

`ComboSourceConfig.groups[]` becomes the single source of truth for group
count, item counts and size weights. At apply time the frame's
`group_count / group_item_counts / group_size_weights` are *derived* from it
(still serialised, so old binaries read the file). `group_item_orientations`
is a layout concern and stays on the frame (edited at the `Layout` node).
Reorder keeps using `combo::reorder` so both key spaces move together.

### 2.5 Theme library and theme references (the only model growth)

"A group of widgets with a theme" has no backing today. Proposed, all
additive with `#[serde(default)]`, so every existing v2 file loads unchanged:

```rust
// crates/types/src/theme.rs
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ThemeRef {
    #[default]
    Inherit,                          // panel → group → global
    Global,
    Named { name: String },           // AppConfig.themes[name]
    Inline { theme: ComboThemeConfig } // what today's combo frame.theme becomes
}

// AppConfig
#[serde(default)] pub themes: BTreeMap<String, ComboThemeConfig>,
#[serde(default)] pub groups: Vec<PanelGroup>,

pub struct PanelGroup {
    pub id: String,
    pub name: String,
    pub theme: ThemeRef,
    pub appearance: Option<PanelAppearance>,   // optional shared look
}

// PanelConfigV2
#[serde(default)] pub group_id: Option<String>,
#[serde(default)] pub theme: ThemeRef,
```

Resolution: `panel.theme` → `group.theme` → `global_theme`. Migration on
load is unconditional: every existing combo panel (any file without a
`theme` field) becomes `Inline { theme: frame.theme }`, so nothing renders
differently after the upgrade, even a combo whose theme happens to equal the
defaults. Only newly created panels start as `Inherit`. On save the resolved
theme is still written into `frame.theme` so older binaries render correctly.

Rendering change is small: the `global_theme` key that `src/main.rs:648`
and `src/ui/window_settings_dialog.rs:974` push into every standalone
displayer (read by `src/displayers/mod.rs:78 apply_global_theme`) becomes
"push the resolved theme for this panel", and combo widgets receive the
resolved theme where they read `frame.theme` today. The hard-coded presets
(`ComboThemeConfig::default_for_lcars()` etc.) seed the library as read-only
built-ins the user can duplicate.

Note on the word "group": this design uses *Group* for a set of standalone
panels sharing a theme (and optionally appearance), and *Content group* for
the rows inside a combo panel. The user's request ("group of widgets (with a
theme)") is read as the former.

### 2.6 Live edit or batched Apply: a real fork

Today the dialog batches everything until Apply/Accept, and each editor paints
its own preview. A tree inspector feels right when edits are live, and
Option B (modeless outliner) needs live edits. The costs differ by node:

| Change | Cost | Proposed behaviour |
|---|---|---|
| displayer/appearance/background field | `apply_config_typed` + `queue_draw`, sub-ms | live, debounced ~50 ms |
| geometry | collision check | live on spin commit; collision reverts size only, as today |
| source *field* on a standalone panel (unit, limits, sensor) | `SharedSourceManager::configure_source` on the existing instance | live |
| source *field* on a combo item | today `ComboSource::configure` calls `setup_child_sources`, which clears and re-creates **every** child (`crates/sources/src/combo.rs:57`; CLAUDE.md gotcha 19) | explicit commit **unless** an incremental configure is added that diffs slots and reconfigures same-type children in place; that is new work, not in the current Apply path |
| source *type* or combo structure (new child sources) | instantiates sources, can be seconds | explicit: commit on dropdown change only after a short settle, or an "Apply" on that node |

Snapshot `PanelConfigV2` when the dialog/inspector opens; **Revert** restores
it. Accept closes. This also makes Paste Style consistent (it is just another
live edit that Revert undoes) and lets the in-editor preview `DrawingArea`s
become optional.

If batched Apply is preferred, every option still works: editors hold working
copies and `store()` runs on Apply, exactly as now.

---

## 3. Option A: Tree Properties Dialog (per panel)

Same per-panel dialog, same Apply/Accept/Cancel buttons if wanted. The
five-tab notebook and every nested notebook are replaced by a tree on the
left and one editor on the right, with a breadcrumb. Leaf editors keep their
own inner tabs (Bar's `Foreground | Background | …`), so the deepest path is
tree + one row of tabs instead of seven.

```
┌─ Panel Properties — System (LCARS) ──────────────────────────────────────────┐
│ System › Content › Group 2 › Item 2                 [Copy] [Paste] [Reset] [⋯]│
├──────────────────────────────┬───────────────────────────────────────────────┤
│ ▾ System   combination→LCARS │ Item 2 of Group 2            caption: CPU Temp │
│    Position & Size           │ ┌─ Source ──────────────────────────────────┐ │
│    Theme        LCARS Classic│ │ Source  [CPU             ▾]  [Copy][Paste] │ │
│    Frame                     │ │ Caption override [CPU Temp           ]     │ │
│    Headers                   │ │ Field   [Temperature     ▾]                │ │
│    Segments                  │ │ Sensor  [Tctl            ▾]  Unit [°C ▾]   │ │
│    Layout                    │ │ Limits  ☑ auto   min [ 0 ]  max [ 100 ]    │ │
│    Animation                 │ └───────────────────────────────────────────┘ │
│  ▾ Content                   │ ┌─ Display ─────────────────────────────────┐ │
│    ▸ Group 1   1 item        │ │ Display as [Bar ▾]   ☑ auto height  [60]  │ │
│    ▾ Group 2   3 items       │ │ ┌Bar Style┬Foreground┬Background┬Border┬…┐ │ │
│        Item 1  CPU Usage→Bar │ │ │                                        │ │ │
│      ▸ Item 2  CPU Temp →Bar │ │ │   existing BarConfigWidget, unchanged  │ │ │
│        Item 3  Memory   →Bar │ │ │                                        │ │ │
│    ▸ Group 3   3 items       │ │ └────────────────────────────────────────┘ │ │
│    ▸ Group 4   7 items       │ └───────────────────────────────────────────┘ │
│      …                       │                                               │
│    Background                │                                               │
│    Border & Corners          │                                               │
├──────────────────────────────┴───────────────────────────────────────────────┤
│                                                [Cancel]   [Apply]   [Accept] │
└──────────────────────────────────────────────────────────────────────────────┘
```

A standalone panel's tree is just:
`Panel › Position & Size · Source (CPU) · Display (Arc) · Background · Border & Corners`.

Interaction details:

* **Item node = one page, two sections.** Source on top (dropdown + the
  source's editor), display below (Display-as dropdown + the display editor).
  Changing the source re-pushes fields into the display section in place.
  One combo-compatible displayer has no per-item display config:
  `css_template` (`crates/sources/src/lib.rs:100` lists the eleven
  compatible displayers; the other ten all carry `content_items`). For it the
  item node shows the Source section only, and the placeholder-to-slot
  mappings stay on the Display node where the template editor lives today.
* **Content group node** edits item count, size weight, orientation; has
  Add item / Remove last / Move up / Move down; so does the context menu on
  any row. Drag-and-drop reorder in the tree is a later nicety.
* **Node context menu** (right-click any row): Copy, Paste, Reset to default,
  Save as default (on the panel/display node, writes `defaults.json`),
  Save to file (panel node), Duplicate (items/groups). This is the one home
  for the typed clipboard; the per-editor Copy/Paste buttons can stay.
* **Themes**: the `Theme` node offers `Inherit / Global / Named / Inline`
  plus the existing 4-colour + gradient + 2-font editor when Inline, and an
  `Edit theme…` link that opens the theme library (a new small dialog that
  replaces Window Settings › Global Theme).
* **Switching displayer** at the Display node keeps using
  `TransferableComboConfig`, so layout and content survive an LCARS →
  Material switch as they do today.
* **Groups of panels**: not in this tree (the dialog shows one panel).
  Membership and group themes are managed by drag-and-drop in the separate
  Themes & Groups window (§9.1); the panel's root node shows its group and
  resolved theme read-only with a button to open that window.

What changes in code: `grid_properties_dialog.rs` (5.2 k lines),
`ComboSourceConfigWidget::rebuild_tabs`/`create_slot_tab` (≈1 k), and
`combo_config_base::rebuild_content_tabs`/`create_content_item_config`
(≈1 k) are replaced by roughly 3–4 k lines of `src/ui/config_tree/`
(node model, tree view, editor host, model builder, apply). Of the 34
`*_config_widget.rs` files, the ten themed combo widgets are restructured:
they lose their Content tab and their remaining pages (Theme/Frame/Headers/
Segments/Layout/Animation) are mounted under `DisplaySection` nodes. The
other 24 (source editors, bar/arc/graph/text/speedometer/etc., background,
gradient) are untouched apart from the `NodeEditor` adapter.

Fits: someone who mostly configures one panel at a time and wants the fastest
large improvement with the least risk.

---

## 4. Option B: Dashboard Outliner (whole dashboard)

Everything in Option A, but the tree is the entire dashboard and lives in a
modeless side pane (or a dockable window) with two-way selection: click a
panel in the grid and its node is revealed; click a node and the panel is
highlighted. Window & grid settings, themes, defaults, timers and panel
groups become nodes, so Window Settings and the theme editor fold into the
same place.

```
┌─ rg-Sens ────────────────────────────────────────────────────────────────────┐
│┌ Outliner ──────────────┐┌ Dashboard ────────────────┐┌ Inspector ──────────┐│
││ 🔍 [ filter…          ] ││ ┌─────┐ ┌─────┐ ┌─────┐  ││ Group "Left column" ││
││ ▾ Dashboard             ││ │LCARS│ │MATL │ │RETRO│  ││                     ││
││    Window & Grid        ││ │     │ │     │ │     │  ││ Theme [My Blue   ▾] ││
││  ▾ Themes               ││ │     │ │     │ │     │  ││       [Edit theme…] ││
││      Global             ││ └─────┘ └─────┘ └─────┘  ││                     ││
││      LCARS Classic  ◦   ││ ┏━━━━┓ ┏━━━━┓            ││ Shared appearance   ││
││      My Blue        ◦   ││ ┃GPU ┃ ┃RAM ┃ ←selected  ││ ☑ corner radius [10]││
││    Defaults             ││ ┗━━━━┛ ┗━━━━┛            ││ ☐ border            ││
││    Timers & Alarms      ││                          ││ ☑ background  [■ ▾] ││
││  ▾ Panels               ││                          ││                     ││
││    ▸ System      LCARS  ││                          ││ Members (2)         ││
││    ▸ Material           ││                          ││   GPU Load   [×]    ││
││    ▸ Retro              ││                          ││   RAM        [×]    ││
││    ▾ ▦ Left column      ││                          ││ [+ Add selected]    ││
││        ▸ GPU Load  arc  ││                          ││                     ││
││        ▸ RAM       bar  ││                          ││ [Revert]            ││
│└────────────────────────┘└──────────────────────────┘└─────────────────────┘│
└──────────────────────────────────────────────────────────────────────────────┘
```

Expanding `System` gives exactly Option A's subtree. Selecting several panel
nodes (or box-selecting in the grid) shows a **multi-edit** inspector with the
sections the selection has in common (Theme, Appearance, Position, Source if
same type) and writes to all of them; this replaces Copy/Paste Style for the
common case, though Copy/Paste remains.

Extras this unlocks:

* **Filter box** over the tree (`gpu`, `none`, `→Bar`) to find every item
  using a source, every empty slot, every bar.
* **Drag a panel node** into a group to join it; drag a theme onto a panel or
  group to assign it.
* **Add panel** from the tree (`Panels › +`), pre-filled from defaults;
  New Panel dialog becomes optional.
* **Save/Load Layout, Save Panel to File, Load Panel** as context actions on
  `Dashboard` and `Panel` nodes.

Costs beyond A:

* The panel-group model (§2.5) is required, not optional.
* Live edit (§2.6) is required; a modeless pane cannot sensibly batch. For
  combo items that means the incremental `ComboSource::configure` (diff
  slots, reconfigure same-type children in place) is required too, or item
  source edits stay on an explicit commit.
* Main-window layout changes (a `Paned` around the existing
  `ScrolledWindow › Overlay`), interaction with fullscreen/auto-hide header
  and auto-scroll needs care; the pane should hide in fullscreen by default.
* Two-way selection sync touches `grid_layout.rs` selection state.

Fits: someone who thinks of the dashboard as a whole, wants to see all 90
items at once, and wants themes shared across many standalone panels.

---

## 5. Option C: Property Tree (schema-driven editors)

Options A and B stop the tree at "one editor widget". Option C keeps going:
every section and field inside an editor is itself a node, and the right-hand
pane is a generated property grid rather than a hand-built widget. This is
what Qt Designer, Unity and GTK Inspector do.

```
┌─ Inspector ──────────────────────────────────────────────────────────────────┐
│ 🔍 [ gradient                     ]   3 matches                               │
├───────────────────────────────────────┬──────────────────────────────────────┤
│ ▾ System › Content › Group 2 › Item 2 │ Property            Value          ⟲ │
│   ▾ Display (Bar)                     │ ▾ Foreground                         │
│     ▾ Foreground                      │   Fill type        [Gradient     ▾]  │
│         Fill type                     │   Gradient         ▓▓▒▒░░  [edit…]   │
│         Gradient            ● match   │     angle          [ 90        ]     │
│         Theme link                    │     stops (3)                   [+]  │
│     ▸ Background                      │       0.00   ■ C1 (theme)        ⟲   │
│     ▸ Border                          │       0.50   ■ #ff8800              │
│     ▸ Animation                       │       1.00   ■ C3 (theme)        ⟲   │
│   ▸ Source (CPU)                      │ ▸ Background                         │
└───────────────────────────────────────┴──────────────────────────────────────┘
```

Each config struct gains a declarative schema (a derive macro or a
hand-written `fn schema() -> Vec<Prop>` per type) describing label, group,
kind (number with range, enum, colour-or-theme, font-or-theme, gradient,
text, path, field-picker, sensor-picker, list-of-struct), help text and
default. The property grid renders rows from that schema; complex kinds use
the existing custom widgets as **field editors** (gradient editor, position
grid, sensor picker, text-line stack, background editor), so nothing visual
is lost.

What this adds over A/B:

* **Search across every setting** by name, with the tree filtered to matches.
* **Uniform behaviour at every field**: reset-to-default (`⟲`), theme-link
  toggle on every colour and font, copy/paste on every value, "show only
  changed from default".
* **Consistency for free**: the gradient copy/paste that was forgotten on
  the speedometer track (docs/combo-panel-lessons-learned.md §5) cannot be
  forgotten because the grid supplies it.
* Long term, the 35 k lines of hand-built editors shrink to schemas plus a
  dozen field editors.

Costs:

* It is a rewrite of the editing layer, not just the navigation layer. Done
  incrementally (schema for simple structs first, hand-built widgets kept as
  opaque field editors until replaced) it is months of work, with regression
  risk on the hand-tuned previews and the special-case behaviours scattered
  through the editors (percent scaling on paste, sensor-label identity,
  combined-text groups, segment↔group sync).
* The `ConfigSchema`/`ConfigOption` stubs on the `Displayer` trait are a
  starting hook but are far too thin (flat key/type/default) to drive this.
* Property grids trade discoverability for density: beginners find a
  purpose-built "Bar Style" page easier than a 40-row grid.

Fits: as a *later* evolution of A or B once the tree shell exists, or now if
the priority is a single uniform editing model and a search box over all
settings.

---

## 6. Functionality preservation checklist

Every current capability and where it lives in each option.

| Capability (today) | A: Tree dialog | B: Outliner | C: Property tree |
|---|---|---|---|
| Size, scale/offset, z-index, ignore-collision | Geometry node | same | same, as rows |
| Every source editor (11) | Source node / Item node | same | schema or opaque field editor |
| Every displayer editor (20) incl. inner tabs | Display node (inner tabs kept) | same | tabs become child nodes |
| Combo: Theme/Frame/Headers/Segments/Layout/Animation pages | `DisplaySection` nodes | same | sections as nodes |
| Combo: per-item display-as + 6 sub-editors | Item node, Display section | same | Item › Display subtree |
| Combo: per-item source + config + caption override | Item node, Source section | same | Item › Source subtree |
| Group count / item count / size weight / orientation | Content + Group nodes (single owner: source) | same | same |
| Item/group reorder, keys kept in step | Group node buttons + row context menu | + drag-and-drop | same |
| Field metadata → display editors, background field picker, CSS mappings | pushed per item on source change | same | same |
| Switch themed displayer keeping layout/content | Display node, `TransferableComboConfig` | same | same |
| Background editor using the panel's own theme (b03b0b6) | Background node gets resolved theme | same | same |
| Corner radius, border | Appearance node | same | rows |
| Copy / Paste Style (context menu + Appearance tab) | row context menu on Panel/Appearance + grid context menu | + multi-edit | same |
| Paste Style writes live, Cancel does not revert | becomes a live edit covered by Revert (or stays as-is if batched) | live + Revert | same |
| Set as Default Style → `defaults.json` | "Save as default" on Display/Panel node + grid menu | same + Defaults node lists them | same |
| Defaults list / general defaults (Window Settings) | unchanged dialog | Defaults node | Defaults node |
| Typed clipboard inside editors (gradient, text line, font, colour, bar/graph/arc/speedo/text, cpu/gpu, background, source) | per-editor buttons kept, plus node Copy/Paste | same | uniform per-row copy/paste |
| Save Panel to File / Load Panel / Save/Load Layout | grid context menu as today | + node context actions | same |
| Theme presets per themed displayer | Theme node › Preset dropdown; seeds the library | Themes node (built-ins) | same |
| Global theme editing | theme library dialog | Themes › Global node | same |
| Multi-select move/delete/copy/duplicate in grid | unchanged | unchanged + multi-edit | same |
| Apply / Accept / Cancel, collision reverts size only | kept (batched) or live + Revert | live + Revert | live + Revert |
| Per-panel dialog cache, scroll preservation | kept | pane is persistent, cache unneeded | same |
| Timers / alarms dialog | unchanged | Timers & Alarms node | same |
| Window mode, renderer, auto-scroll, viewport | unchanged dialog | Window & Grid node | same |
| New Panel dialog | unchanged | optional; `Panels › +` | same |
| **New:** named themes, panel groups sharing a theme | model + small dialog; assign via grid context menu | first-class nodes, drag to assign, multi-edit | same |

---

## 7. Effort, risk, recommendation

| | A: Tree dialog | B: Outliner | C: Property tree |
|---|---|---|---|
| Code replaced | ~7 k lines of composition | A + main-window pane + selection sync | A/B + progressive rewrite of 35 k lines of editors |
| Code written | ~3–4 k (`config_tree/`) + theme library ~1 k | A + ~2–3 k (outliner, groups, multi-edit) | B + schema layer + property grid + field editors, ongoing |
| Model changes | `ThemeRef` + `themes` (additive) | + `PanelGroup`, `group_id` (additive) | same as B |
| Regression risk | low: leaf editors untouched | medium: main window, selection, live edit everywhere | high initially, falls as editors are replaced |
| Opens panel with 29 items as fast as today | yes (lazy editors) | yes | yes if rows are built lazily per section |
| Answers "group of widgets with a theme" | partially (assign from grid) | fully | fully |
| Answers "overview of everything" | per panel | yes | yes + search |

**Recommendation: build A as phase 1, designed so B is phase 2.** The node
model, editor factory, theme library and live-edit path are identical in A
and B; A proves them on one panel inside the existing dialog frame with the
least blast radius. B then adds the outliner pane and groups on top without
reworking anything. C is worth revisiting afterwards for the search box and
per-field theme linking, and can be introduced one config type at a time
because the property grid is just another `NodeEditor`.

Suggested phasing:

0. **Theme library + `ThemeRef`** (independent, small): model, migration,
   resolution, Themes dialog replacing Window Settings › Global Theme. Gives
   per-panel named themes immediately, even before groups exist.
1. **Tree shell**: `src/ui/config_tree/` (node model, `TreeListModel`
   view, breadcrumb, lazy editor host, model builder, apply). Mount every
   existing editor via `NodeEditor` adapters. Ship as the Properties dialog;
   keep the old file until parity is verified against the checklist in §6,
   then delete it.
2. **Unified item node + single structure owner**: retire
   `rebuild_content_tabs`, `rebuild_tabs`, the `on_fields_updated` /
   `set_source_summaries` / `apply_reorder` matches.
3. **Groups + outliner** (Option B) or **schema editors** (Option C).

Optional, separate from the UI: change `ContentItemConfig` to hold only the
chosen display's config (an enum keyed by `display_as`), with a
deserialise-both migration. Cuts the author's config from 4.2 MB to well
under 1 MB and makes the item node map 1:1 onto the data. It is a format
change and should be its own commit with a round-trip test.

---

## 8. Decisions (taken 2026-11-05)

1. **Option A first.** Option B is rejected: a persistent outliner pane does
   not fit small windows. **Option C is the long-term target**, so the tree
   shell is built with the property grid in mind (§9.5).
2. **Live edit with Revert** (§9.2). No Apply/Accept.
3. **Item node is one page** with Source and Display sections (§9.4).
4. **Group membership by drag-and-drop in a tree** (§9.1).
5. **`ContentItemConfig` keeps its format.** All six sub-configs stay
   stored per item; the UI simply builds only the selected one.

---

## 9. Detailed design for Option A

### 9.1 Themes & Groups window (where groups live)

The Properties dialog shows one panel, so panel groups and the theme library
get their own small tree window. It replaces Window Settings › Global Theme
and is the Phase 0 deliverable.

```
┌─ Themes & Groups ────────────────────────────────────────────────────┐
│ ▾ Themes                         │ Theme "My Blue"                   │
│     Global                       │ ┌──────────────────────────────┐  │
│     LCARS Classic    (built-in)  │ │ existing GlobalThemeWidget   │  │
│     Material Dark    (built-in)  │ │ C1 C2 C3 C4 · gradient       │  │
│     My Blue                      │ │ font 1 · font 2              │  │
│ ▾ Groups                         │ └──────────────────────────────┘  │
│   ▾ Left column     [My Blue]    │ Used by: Left column, Retro       │
│       GPU Load                   │ [Duplicate]  [Rename]  [Delete]   │
│       RAM                        │                                   │
│   ▾ Right column    [Global]     │                                   │
│       Net                        │                                   │
│ ▾ Ungrouped                      │                                   │
│     System          LCARS inline │                                   │
│     Material                     │                                   │
│ [+ Theme] [+ Group]              │                                   │
└──────────────────────────────────┴───────────────────────────────────┘
```

* Drag a panel row into a group, or into Ungrouped, to set `group_id`.
* Drag a theme row onto a group or a panel to set `ThemeRef::Named`.
  Dropping onto a panel whose theme is `Inline` asks before replacing it.
* Group editor: name, theme (Global or Named). Optional shared appearance
  (corner radius, border, background) is a later addition; the model
  reserves `PanelGroup.appearance` for it.
* Theme editor: the existing `GlobalThemeWidget`, plus Duplicate, Rename,
  Delete (refused while in use). The built-in presets
  (`ComboThemeConfig::default_for_*`) appear read-only; Duplicate makes an
  editable copy.
* Live: editing a theme re-pushes the resolved theme to every panel that
  resolves to it, through the same path `src/main.rs:648` uses today.
* Implementation: `TreeListModel` over `ListStore<ConfigNode>`, a
  `DragSource` bound in the row factory, a `DropTarget` accepting the
  `ConfigNode` GType on group/panel rows.

In Panel Properties every panel, standalone or combo, gets a `Theme` node:
a chooser (Inherit / Global / Named ▾ / Inline), the inline editor when
Inline, the preset dropdown for combos (a preset sets Inline), and a
"Themes & Groups…" button. The panel root node shows its group read-only.

### 9.2 Live edit and Revert

An `EditSession` per dialog holds a `snapshot: PanelConfigV2` (geometry,
source, displayer, appearance, theme, group) taken when the dialog opens.
Editors call `session.changed(node)`; a 50 ms coalescing debounce
(`glib::timeout_add_local_once`) then runs `commit(node)`:

| Node | Commit path |
|---|---|
| Display, DisplaySection, Content, ContentGroup, item Display section, Theme | rebuild the typed `DisplayerConfig` from editors → `displayer.apply_config_typed` → `queue_draw` |
| Background | `panel.background` + `background_area.queue_draw` |
| Appearance | corner radius / border through the existing per-panel CSS provider |
| Geometry | existing collision-checked resize; collision reverts the size spinners only |
| Source fields, standalone, same type | `SharedSourceManager::configure_source` (configures the live instance in place; `src/core/shared_source_manager.rs:508`) |
| Source type or Display type change | existing Apply logic (create, swap widget reusing the overlay), run immediately on dropdown change, no debounce |
| Item Source section (combo) | `configure_source` with the whole `ComboSourceConfig`; needs the incremental configure of §9.3; the item's field metadata is recomputed async and pushed to its Display section |
| Theme / group | resolve and push |

Every commit fires the dialog's existing `on_change` so the dirty flag and
save path are unchanged. **Revert** re-applies the snapshot through the same
commit paths (including type changes) and reloads every built editor from
it. **Close** closes. Paste Style and Reset-to-default become ordinary
edits, so Revert covers them too; the old "Cancel does not revert Paste
Style" quirk disappears. Editor preview `DrawingArea`s are kept for now (they
still serve the Themes window); Phase 3 may drop them.

### 9.3 Incremental `ComboSource::configure`

Today `configure` replaces `self.config` and calls `setup_child_sources`,
which clears `child_sources` and recreates every child
(`crates/sources/src/combo.rs:57`). Live edit needs a diff by slot name:

| old → new | action |
|---|---|
| same `source_id`, same `source_config` | keep child |
| same `source_id`, different `source_config` | `child.configure(&new.source_config)` |
| different `source_id` (including `none`) | drop child, create new |
| slot removed or group shrunk | drop child |
| slot added | create |

`get_slot_names()` already derives slot names from `groups`. This also
removes the "3 s stall after every Apply" recorded in CLAUDE.md gotcha 19.
Unit-testable without GTK.

### 9.4 Item node page

```
System › Content › Group 2 › Item 2
Caption override [                 ]        Order [↑] [↓]
▾ Source ─ CPU ───────────────────────────────────────────
   Source [CPU ▾]                            [Copy] [Paste]
   <CpuSourceConfigWidget>
▾ Display ─ Bar ──────────────────────────────────────────
   Display as [Bar ▾]   ☑ auto height   Height [60]
   <BarConfigWidget, inner tabs intact>
```

Both sections are GTK `Expander`s, open by default. Only the editor for the
selected source and the selected display type is built; changing a dropdown
builds the new editor lazily and hides the old one, which stays alive
because its config is still stored (decision 5). For `css_template` the
page has the Source section only.

### 9.5 Node model and the road to C

* `NodeKind` as in §2.1 plus `Section { path: Vec<String> }`, reserved so
  an editor can later expose its own sub-sections (`Bar › Foreground`) as
  child nodes without changes to the tree code.
* `NodeEditor` as in §2.2 plus
  `fn sections(&self) -> Vec<(String, gtk4::Widget)>` (default empty). A
  schema-driven editor returns its sections and the tree shows them as
  children; a legacy editor returns nothing and is shown whole. This is the
  seam that turns A into C one config type at a time.
* Tree: `TreeListModel` (autoexpand off) over `ListStore<ConfigNode>`,
  `ListView` + `SingleSelection`, `SignalListItemFactory` binding a
  `TreeExpander`, icon, label, dim summary. Children are built on first
  expand.
* Breadcrumb: a row of link buttons rebuilt on selection; a crumb selects
  its node.
* Editor host: a `Stack` with one page per node id, built on first
  selection, all `cleanup()`ed on close. The per-panel dialog cache stays.
* Row context menu (`PopoverMenu` from a `gio::Menu`, same pattern as the
  panel menu): Copy, Paste, Reset to default, Save as default (panel and
  display nodes), Save panel to file (panel node), Move up/down and
  Duplicate (groups and items).

### 9.6 Phase plan, file level

**Phase 0: theme library and groups** (independent of the tree)

* `crates/types/src/theme.rs`: `ThemeRef`, `PanelGroup`; serde round-trip
  tests.
* `src/config/settings.rs`: `AppConfig.themes`, `.groups`;
  `PanelConfigV2.theme`, `.group_id`; unconditional Inline migration for
  existing combos; `resolve_theme(panel) -> ComboThemeConfig`; tests.
* `src/main.rs:648`, `src/ui/window_settings_dialog.rs:974`: push the
  resolved theme per panel (for combos into `frame.theme` before
  `apply_config_typed`).
* `src/ui/themes_groups_dialog.rs` (new): tree, drag-and-drop,
  `GlobalThemeWidget` reuse. Window Settings › Global Theme becomes a
  button that opens it.

**Phase 1: tree shell for Panel Properties**

* `crates/sources/src/combo.rs`: incremental configure (§9.3) + tests.
* `src/ui/config_tree/{mod,node,tree,breadcrumb,editor,host,builder,session,dialog}.rs`.
* `src/ui/config_tree/editors/`: one adapter per existing widget: 11
  sources, 20 displayers (the ten themed ones expose their pages minus
  Content as `DisplaySection` nodes), background, appearance, geometry,
  theme, content group, item.
* `src/ui/editor_registry.rs`: `register_source_editor`,
  `register_displayer_editor`, called from `src/ui/mod.rs` at startup.
* `grid_layout.rs`: the `panel.properties` action opens the new dialog.
  `grid_properties_dialog.rs` stays until the §6 checklist passes, then is
  deleted.

**Phase 2: single owner and dead-code removal**

* Derive frame `group_count / group_item_counts / group_size_weights` from
  `ComboSourceConfig.groups` at commit.
* Delete the `set_source_summaries` fan-out, the `on_fields_updated` and
  `apply_reorder` matches, `ComboSourceConfigWidget::rebuild_tabs`,
  `combo_config_base::rebuild_content_tabs` /
  `create_content_item_config`, and the Content tab in the ten themed
  widgets.

**Phase 3: toward C**

* `crates/types`: `Prop` descriptors (`fn props() -> Vec<Prop>` per config
  type; a derive macro later).
* `src/ui/config_tree/schema_editor.rs`: `SchemaEditor: NodeEditor`
  rendering rows from `Prop`s with field editors (number, enum,
  colour-or-theme, font-or-theme, gradient, field picker, sensor picker,
  list-of-struct), with `sections()` implemented.
* Migrate one config type at a time, simplest first (memory, disk, network,
  claude, static_text sources), then bar/arc/graph/text/speedometer, then
  the themed frames. Add the tree search box once a few types are
  schema-driven.

---

## 10. Phase 0 as shipped (2026-11-05)

What exists after Phase 0:

* `crates/types/src/theme.rs`: `ThemeRef`, `PanelGroup`, `ThemeLibrary`
  (resolver), `ComboThemeConfig::builtin_presets()`. The missing-named-theme
  policy lives in `ThemeLibrary::resolve_missing_named` and is provisional
  (falls back to the global theme); it is the one open policy decision.
* `crates/types/src/panel.rs`: `PanelData.theme`, `PanelData.group_id`,
  `DisplayerConfig::theme()/theme_mut()/has_frame_theme()`.
* `src/config/settings.rs`: `AppConfig.themes`, `AppConfig.groups`,
  `PanelConfigV2.theme/group_id`; key-absence migration to `Inline` for
  combos (v2 and v1 paths); `resolve_theme_for*`, `theme_users`,
  `rename_theme` (rewrites references), `delete_theme` (refused in use),
  groups CRUD, `unique_theme_name`; `sanitize` drops dangling `group_id`.
* `src/core/panel.rs`: `theme_ref/set_theme_ref/group_id/set_group_id`,
  `apply_resolved_theme` (standalone: `global_theme` key; combo: writes
  `frame.theme` and re-applies the typed config), `sync_inline_theme` used by
  `to_data()`.
* `src/ui/config_helpers.rs::apply_resolved_themes`: the one push path,
  used at startup (`main.rs`) and by Window Settings Apply.
* `src/ui/themes_groups_dialog.rs`: the Themes & Groups window (§9.1):
  `TreeListModel`/`ListView` tree, drag panel → group/Ungrouped, drag theme →
  group/panel (confirmation when replacing an Inline theme), theme editor
  (`GlobalThemeWidget`), rename/duplicate/delete, group name + theme, panel
  theme chooser including Inline with an inline editor. Opened from both
  context menus ("Themes & Groups…") and from Window Settings › Themes.
* `grid_layout.rs`: the four background draw sites and the three
  properties-dialog sites now use the panel's resolved theme.

Known Phase-0 limitations, all removed by Phase 1:

* The old Panel Properties dialog edits a combo's `frame.theme` in its
  Theme tab. That works for `Inline` panels (the edit is synced back on
  save) but for a `Named`/`Global` combo the tab shows the *resolved* theme:
  it looks editable, and the next theme push overwrites the edit.
* The Themes window refreshes its `panels_v2` snapshot from the live panels
  (`to_data()`, a clone of each combo's config) on every selection change and
  before every structural edit. Cheap at a handful of panels; Phase 1's node
  model should keep theme/group/labels without cloning whole configs.
* The global theme is editable in the window but, by design, not renamable
  or deletable; built-in presets are read-only and must be duplicated.

---

## 11. Phase 1 as shipped (2026-11-05)

* `crates/sources/src/combo.rs`: `ComboSource::reconcile_children` (§9.3):
  unchanged slots keep their child instance, config changes reconfigure in
  place, type changes recreate one slot. Tested.
* `src/core/shared_source_manager.rs::reconfigure_or_recreate`: a panel that
  is its source's only user keeps the live instance across config changes
  (re-keyed), so a combo keeps its children through live edits. Used by
  `Panel::update_from_data` when the source type is unchanged.
* `src/ui/editor_registry.rs`: `SourceEditor` / `DisplayEditor` /
  `ComboDisplayEditor` traits, adapters over all 11 source widgets and 20
  displayer widgets (css_template behind the `webkit` feature), registered
  by id in `register_all_editors()`. Themed combo editors detach their
  notebook pages (`take_pages`) and build per-item editors bound to their
  shared config (`item_editor`, via `create_content_item_config`).
* `GridLayout::apply_panel_data` (`grid_layout.rs`): the one path from a
  `PanelData` to grid state. The grid publishes a weak handle
  (`grid_layout::grid_weak()`) so dialogs opened from any of the three
  panel-menu construction paths can reach it.
* `src/ui/config_tree/`: the dialog. Tree: Panel › Position & Size, Theme,
  Source (standalone), Display (› sections for combos), Content (› Group ›
  Item, combos), Background, Border & Corners. Live commit after a 50 ms
  debounce; Revert re-applies the opening snapshot and rebuilds the editors;
  Close. Source and clock editors have no change signal and are polled
  (300 ms) while their page is visible. Item node = caption + order, Source
  expander (dropdown, editor, copy/paste), Display expander (display-as and
  the embedded lazily built display editors). Theme node = reference
  chooser; the themed editor's own Theme page (combos) or a
  `GlobalThemeWidget` (standalone) appears for Inline. Display node has the
  displayer dropdown (combo → combo switches carry `TransferableComboConfig`;
  new displayers start from their saved default style) and "Save as Default
  Style". Border & Corners has Copy/Paste Style.
* Context menu: "Properties…" opens the tree dialog on all three
  construction paths; "Properties (classic)…" opens the old dialog until the
  §6 checklist has been verified, then `grid_properties_dialog.rs` goes
  (Phase 2).

Known Phase-1 limitations:

* Group size weights and orientation are edited under Display › Layout (the
  themed editor's page), not on the group node; the source-side
  `GroupConfig.size_weight` is not exposed (rendering uses the frame's).
* Structural changes (group/item count, reorder, item source change) rebuild
  the Content subtree and drop its item pages; per-item display editors are
  recreated on next selection.
* No row context menu yet (copy/paste/reset per node); the per-editor
  buttons and the grid context menu cover those actions.
* The Themes & Groups window is not linked from the Theme node (it needs
  handles the dialog does not hold); open it from the dashboard menu.
* The Themes & Groups window and this dialog can both be open; a theme
  assigned by drag there is overwritten by this dialog's own theme choice
  on its next commit. Likewise the classic and tree dialogs can be open for
  the same panel at once; the last edit wins.
* Per-item theme refreshers and cleanup callbacks accumulate across
  structural changes until the window closes (bounded by one session).
* Not exercised by the author in a running GTK session (no headless display
  available while the user's instance was running): the §6 checklist is the
  acceptance test.
* Window Settings' background widgets receive the global theme when the
  dialog opens; theme edits made in the Themes window while it is open show
  up on reopen.
* Live edit of a combo item's *source* is still batched (needs §9.3).
