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
  chooser over an always-visible theme editor (the themed editor's own Theme
  page for combos, a `GlobalThemeWidget` for standalone panels): a shared
  reference shows the resolved theme and any edit to it makes the panel's
  theme `Inline` (see §11.1). Display node has the
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

### 11.1 First GUI feedback (2026-11-05, same day)

The user's first run found the Theme node unusable for new panels: it showed
only the reference dropdown, and the colours shown for existing combos went
stale after a dropdown change. Three defects, fixed together:

* **Theme editor hidden unless Inline.** The editor is now always on the
  page. For Inherit / Global / Named it shows the resolved theme; for Inline
  the private one. Editing a colour, gradient or font under a shared
  reference detaches the panel: `State::detach_theme_if_edited` runs before
  every commit and compares the editor payload with `theme_baseline`, the
  value last pushed into the editor (not with the current resolution, so a
  theme changed in the Themes & Groups window meanwhile does not detach the
  panel). The dropdown jumps to Inline and the row summary follows. Picking
  a built-in preset keeps a `Named` reference; tweaking it afterwards makes
  it Inline with the preset's values plus the tweak. Library themes are never
  edited from this page — that stays in the Themes & Groups window. After a
  successful apply, a baseline that no longer matches the resolution
  re-propagates, so an Inline edit reaches the background and item editors
  and an external change reaches the theme editor. That re-propagation also
  calls the themed widget's `set_theme` with its own payload: the widgets
  refresh their Display-section colour and font selectors (LCARS headers,
  segments, divider; the other widgets' registered refreshers) only there.
  To make that self-push harmless, `GradientEditor::set_gradient_source_config`
  returns early when the gradient is unchanged instead of rebuilding the stop
  rows under the pointer; the colour buttons and font controls were already
  signal-free on unchanged values. (Second GUI round: without the self-push
  the Display sections kept showing the inherited swatches after a Theme-tab
  edit.)
* **`rebuild_content_tabs` dropped the page-level theme refreshers.** Third
  GUI round (Art Deco: Theme › Color preset changed the panel and the item
  editors, but Display › Frame / Background / Header kept the inherited
  swatches). The themed widgets' Frame/Header/Background selectors follow
  theme changes only through refreshers registered at construction, and the
  generic content rebuild cleared that list on every call, keeping just its
  first entry — so after the first `set_config` or `set_source_summaries`
  those selectors were frozen. LCARS was immune because its `set_theme`
  updates its selectors explicitly. Content items now register into a list
  of their own behind one forwarding entry, which the rebuild's cleanup
  removes; the page-level entries are never touched. Pre-existing in the
  classic dialog too (visible after any Apply).
* **Themed widgets' `set_theme` did not refresh their Theme tab.** Only
  `set_config` updated the C1–C4 buttons, gradient editor and font controls,
  so a theme pushed by the dialog left the tab showing the previous values.
  Each of the ten widgets now has `refresh_theme_tab`, called from both
  `set_theme` (with `on_change` suppressed — the size spin buttons emit
  `value-changed` on `set_value`) and `set_config`. The five
  `CommonThemeWidgets`-based widgets use `apply_theme_preset`, which also
  fixes a latent bug: `set_config` never updated the gradient editor's
  `current_colors`, so the first colour edit after a load re-synced the
  gradient from construction-time colours.
* **New panels never received a resolved theme.** `GridLayout::add_panel`
  now pushes the resolved theme before creating the widget; `main.rs` no
  longer needs its startup pass (`config_helpers::apply_resolved_themes`
  stays for the Themes & Groups window). Until then a new combo rendered
  with the displayer's default frame theme until the first edit or restart,
  and the dialog's first commit would have detached such a panel.

Open points: nested scrolling where a themed editor's Theme page carries its
own `ScrolledWindow` (Art Deco, Art Nouveau, Steampunk) inside the dialog's
scrolled page; a widget-level preset dropdown (Cyberpunk, Industrial, Retro,
Synthwave, Art Deco, Art Nouveau, Steampunk) is not reset when the dialog
pushes a theme, so it may name a preset the colours no longer match (same as
`set_config` today).

---

## 12. Phase 2 as shipped (2026-11-05)

Four commits, each green on its own:

1. **Single owner of group structure.** `ComboSourceConfig.groups` owns
   group count, item counts and size weights; the frame's
   `group_count / group_item_counts / group_size_weights` are a rendering
   cache written by `DisplayerConfig::sync_group_structure`, called from
   `GridLayout::apply_panel_data` and `create_panel_from_data`. Size
   weights had only ever been edited in the frame (the source's stayed at
   the default), so loading adopts frame weights into the source once where
   they differ (`settings.rs::adopt_frame_group_weights`, logged); every
   save after that writes them equal. Verified on the live config: four
   combos adopted, save/reload identical. The group node gained a size
   weight spin (`State::set_group_weight`, live through
   `ComboDisplayEditor::set_group_weights`); the themed editors' Layout page
   keeps only the per-group orientation switch; a structure change reloads
   the themed editor with the synced frame (`store → sync → load`).
2. **Classic dialog deleted.** `grid_properties_dialog.rs`, the
   "Properties (classic)…" entry and its three action blocks. The tree
   dialog took over the two hooks: `config_tree::close_tree_properties`
   (panel deleted) and `close_all_tree_properties` (main window closing).
3. **`ComboSourceConfigWidget` deleted.** `ComboReorder` moved to
   `editor_registry.rs`.
4. **Content tab removed from the ten themed widgets.** The per-slot
   notebook, `rebuild_content_tabs` / `create_content_page` /
   `cancel_pending_content_rebuilds`, `set_source_summaries`,
   `set_available_fields` and the content cleanup list are gone;
   `new()` takes no fields. `create_content_item_config` stays: it is how
   `ComboDisplayEditor::item_editor` builds the per-item editors.
   `DisplayEditor::set_source_summaries` stays on the trait for
   css_template's placeholder mapping.

Net: about 9,500 lines removed. Still open from §11: row context menu,
Themes & Groups window not linked from the Theme node, the Phase-1
"both windows open" caveats, and `ThemeLibrary::resolve_missing_named`.

---

## 13. Phase 3a as shipped (2026-11-05)

The first slice toward Option C: schema infrastructure plus the three
sources that need no custom widget.

* `crates/types/src/schema.rs`: `Prop { key, label, help, section, kind,
  visible_when }`, `PropKind::{Bool, Int, Float, OptionalFloat, Text,
  OptionalText, Enum, DynamicChoice}`, `Condition::{Equals, IsFalse}`, the
  `Schema` trait (`fn props() -> Vec<Prop>`) and `check_props_fit::<T>()`,
  which verifies every key and enum option against the type (the one
  failure a stringly schema hides from the compiler). Enum option values are
  serde variant names verbatim.
* `src/ui/config_tree/schema_editor.rs`: `SchemaEditor` renders rows over
  the flat JSON form of a config (keys without a prop pass through; integers
  stay integers; optional text stores `null` when empty; dynamic choices keep
  a saved value the provider does not list as its own row). Change signal,
  conditional visibility, section headings. `SchemaSourceEditor` implements
  `SourceEditor` through the per-slot flat form, so one editor serves the
  Source node and combo item pages. Choice providers (`network_interfaces`,
  `disk_mounts`) are registered in `register_all_editors()`.
* Memory, disk and network implement `Schema`; their widgets are deleted.
  The dialog now polls only editors that report no change signal (standalone
  and per item).
* Search box above the tree: nodes whose title, summary, schema labels or
  page keywords match stay, with their ancestors; clearing restores the tree.

Next slices, in the doc's order: static_text (list-of-struct kind), clock
(timezone picker as a field editor), claude (sign-in buttons as an editor
footer), the four sensor sources (sensor picker kind honouring label
identity), then bar/arc/graph/text/speedometer, then the themed frames with
`sections()` as child nodes.

## 14. Phase 3b as shipped (2026-11-06)

Every source except `test` (whose values live in the global Test Source
dialog) and `combination` is now schema-driven; seven more widgets are gone.

* `crates/types/src/schema.rs` kinds added: `List { item, new_item,
  min_items, item_label, add_label }` (a `Vec` of structs; the template
  function supplies the appended item), `IndexChoice { provider }` (the
  position in a provider list, stored as an integer), `Custom { editor }`
  (an editor the UI registers by name, allowed to read and write several
  keys) and `Note` (text, no key; `Prop::note`). `check_props_fit` also
  validates `Condition::Equals` values, `IndexChoice` defaults and `List`
  templates (closing the §13 gap).
* `src/ui/config_tree/schema_editor.rs`: rows for the new kinds. List items
  are frames with move/remove buttons, rebuilt on every structural edit
  under the `loading` flag so rebuilt controls never commit. A `Custom`
  editor gets a `FieldContext`: `set(vec![(key, value), …])` is silent while
  the editor loads (normalisation) and fires otherwise.
  `register_field_editor(name, factory)` mirrors `register_choice_provider`;
  `SchemaSourceEditor::create_with_footer` appends stateful UI.
* `src/ui/config_tree/field_editors.rs`: `timezone` (button → 
  `TimezoneDialog`), `fan_sensor` / `temp_sensor` (the `SensorPicker` over
  `sensor_label` + `sensor_index`; on load the saved pair is resolved with
  `resolve_fan_index` / `resolve_sensor_index`, a match is written back
  normalised, no match shows the placeholder row plus warning and leaves the
  saved pair alone — gotcha 18 unchanged), `cpu_core_selection`
  (`"Overall"` | `{"Core": n}`). Providers `cpu_sensors`, `gpus`.
* `src/ui/claude_account.rs`: the sign-in block, now the Claude editor's
  footer. `session_hours` and `max_limit` gained rows (visible for the
  token metrics); they were never editable before.
* Defaults: `FanSpeedConfig`, `SystemTempConfig` and `CpuSourceConfig` now
  default to fixed limits (0/3000, 0/100, 0/100) with auto-detect off. The
  old widgets always saved those values; a config without limits makes the
  displayer normaliser return 0 for any value above 100, so a new fan panel
  would have drawn an empty bar.
* `SensorPicker` no longer takes a title or pre-selects a row; the schema
  row supplies the label and load drives the selection.

Known limits: list items' nested enum options are not validated by
`check_props_fit` (no list has one yet); the static-text "Add line" template
numbers lines by count, so a renamed line can collide with a new one, as
before.

Next: bar/arc/graph/text/speedometer display configs (preview + theme
coupling, so `DisplayEditor` adapters stay until a colour-or-theme kind
exists), then the themed frames with `sections()` as child nodes.

## 15. Phase 3c as shipped (2026-11-06)

The first two display configs, bar and arc, are schema-driven; their
widgets are deleted and the combo item page edits bar/arc items through the
same schema editors.

* `crates/types/src/schema.rs`: `PropKind::Object { props }` (nested
  struct), `PropKind::Variant { tag, variants }` with `Variant { value,
  label, props, template }` (internally tagged enum with payload; choosing a
  variant replaces the whole object with the template, so a `gradient`
  object never keeps a stale `color`), `PropKind::Color` (`ColorSource`),
  `PropKind::ColorStops { linear }` (`Vec<ColorStopSource>`, no angle; a
  sibling Float carries the angle where one exists; `linear` picks the bar-style
  ramp preview over the arc-style sweep), `Condition::IsTrue`. `check_props_fit`
  is recursive over paths: every template is substituted into the default
  and the whole type deserialized, nested props are checked against the
  nested object (or the variant's template / the list's template item).
* `src/ui/config_tree/schema_editor.rs`: one `Nested` mechanism (rows over a
  sub-object written back under its key, or into an array slot) serves
  Object, Variant and List items. `SchemaEditor::set_theme` fans the
  resolved theme out to Color/ColorStops rows, nested rows and custom
  editors' theme hooks, and never fires a change. `SchemaEditor::sectioned`
  splits rows by `Prop.section` into pages; `SchemaDisplayEditor` exposes
  them through the new `DisplayEditor::sections()`, and the tree shows them
  as `DisplaySection` child nodes exactly as for combo frames
  (`ensure_display_editor` fills `display_pages` from either). Field editors
  now return a `FieldEditor { widget, refresh, on_theme, cleanup }` and get
  the slot's `fields` and the current theme in their `FieldContext`.
* `text_overlay` field editor wraps `TextOverlayConfigWidget` over the whole
  nested object; its writes are suppressed while refreshing because the
  widget's `set_config` fires its own change signal.
* `combo_config_base::item_schema_editor` replaces the lazy bar/arc widgets
  in the item page; the Level Bar item type renders from `bar_config` and so
  shares the bar editor.
* Display page: type-tagged "Copy/Paste Display Config" (clipboard
  `copy_display_config`), replacing the buttons the two widgets had; a paste
  re-pushes this panel's resolved theme, since the pasted JSON carries the
  source panel's cached `theme`.
* Behaviour: a bar switched to a gradient fill starts with theme colours
  1 → 2 (the old widget left the editor's stop list, often empty); the
  background gradient template keeps the old two greys; arc's in-widget
  preview is gone (the live panel is the preview; bar's was already hidden);
  the old notebook tabs are tree child nodes.

Known limits: a variant's props must not use `Condition::IsFalse` (a key
absent in another variant's object reads as false, so the row would show);
the clipboard still stores resolved stops, so theme links are lost on paste,
as before.

Next: graph, speedometer and text need plain-RGBA colour, font, field-picker
and text-line kinds (text first, since the three overlays embed it); then the
themed frames with `sections()`.

## 16. Phase 3d as shipped (2026-11-06)

The text displayer and the text overlays are schema-driven.

* `crates/types/src/schema.rs`: `PropKind::Font { legacy }` (`FontSource`,
  reading the `(family, size)` keys named in `legacy` when the source is
  `null`, which is how pre-`font_source` lines are stored; nothing ever
  called `TextLineConfig::migrate`), `PropKind::Field` (a field id chosen
  from the fields the editor was given), `ColorStops { themed }` (plain
  `Vec<ColorStop>` when false — `TextFillType` gradients are not theme-aware).
* `crates/types/src/text.rs`: `TextLineConfig`, `TextDisplayerConfig` and
  `TextOverlayConfig` implement `Schema`; the overlay is `enabled` plus an
  `Object` over the displayer config, whose single prop is the `lines` list.
  Bar and arc use that object for their Text overlay section instead of the
  opaque widget.
* `src/ui/config_tree/field_editors.rs`: `text_position` (the 3×3
  `PositionGridWidget`) for `position` and `combine_alignment`; the
  `text_overlay` editor is gone.
* `src/ui/config_tree/schema_editor.rs`: Font, Field and plain-stops rows;
  `SchemaDisplayEditor::create` builds a single page when the props have at
  most one section, so the text displayer's lines sit on the Display page.
  Lists gain Duplicate (per item), Copy (per item) and Paste (header), with
  the clipboard item tagged by list key so a text line pastes only into a
  text-line list.
* `combo_config_base`: Text items use the overlay schema editor; changing an
  item's display type reloads the bar, arc and text editors from the item,
  because bar and text both edit `bar_config` and the hidden one would
  otherwise write back a stale copy on its next edit (the old widgets had the
  same hazard, mitigated by saving the text widget only under the Text type).
* Behaviour: combine direction/alignment rows show on every grouped line
  (the old widget showed them only on the first line of a group with two or
  more lines at one position; the renderer still reads the first line's);
  the group is a free-text name with "Group 1" as placeholder instead of a
  dropdown of eight presets plus Custom; a new line is the default line
  (field "value"), ungrouped; the fill gradient template is white → grey.
* `TextLineConfigWidget` and `TextOverlayConfigWidget` stay for graph,
  speedometer, core bars, indicator and static.

Next: graph (plain RGBA colour kind for its backgrounds; a plain
family/size font kind for axis labels), speedometer (zones list with plain
colour; bezel `BackgroundConfig` as a field editor), core bars, indicator,
static; then delete the two text widgets; then the themed frames with
`sections()`.

### 16.1 List items as tree nodes (2026-11-06)

A schema display editor built with `SchemaEditor::for_tree` detaches its
list items: each becomes an `EditorPage` with a path (`["lines", "0"]` for
the text displayer, `["Text overlay", "lines", "0"]` inside bar/arc) and the
tree files it under its nearest ancestor page or Display
(`state::resolve_page_node`, tested). The list's own row on the parent page is
one compact line per item (Edit…, move, duplicate, copy, remove) plus
Add/Paste. `DisplayEditor::set_on_sections_changed` reports structural edits
with the path to select; `State::refresh_display_pages` evicts the stale
`sec/…` stack pages, re-reads the pages and rebuilds the tree, or just selects
when pages and widgets are unchanged (an Edit… jump). Titles are read live
through `DisplayEditor::page_title` (`PropKind::List.item_summary`:
"Line 1 · value"), so `refresh_row_texts` after a commit renames the node.
Inline lists (source editors, combo item pages, whose editors are not
`DisplayEditor`s) keep frames, now separated by a rule and titled with the
summary.

## 17. Phase 3e as shipped (2026-11-06)

Every non-themed displayer is schema-driven; the hand-written display
widgets are gone except the two clock widgets and css_template.

* `crates/types/src/schema.rs`: `PropKind::Rgba` (plain `Color`),
  `PropKind::PlainFont { size_key }` (family string + size number in the same
  object). `bar.rs` exports `fill_prop`, `background_prop`, `border_prop` so
  core bars share the bar's variants and templates.
* Schemas: `GraphDisplayConfig` (axes as nested objects with plain label
  fonts, margins object, plain background colours), `SpeedometerConfig`
  (value zones are a list — newly editable, the renderer always drew them;
  bezel background through the `background` field editor),
  `CoreBarsConfig` (core range as clamped numbers 0–255 with help text; the
  old widget clamped the spin to the machine instead), `IndicatorConfig`
  (`indicator_shape` field editor for the externally tagged shape; text lines
  nested under Text), `StaticDisplayConfig` (combo items only).
* `src/ui/config_tree/field_editors.rs`: `background` wraps
  `BackgroundConfigWidget` as a block (slot fields, theme, writes suppressed
  while refreshing, change closure dropped on cleanup); `indicator_shape`
  is a dropdown of the common shapes plus a side count.
* `combo_config_base`: graph, core bars, speedometer and static items use
  `item_schema_editor`; a display type switch reloads all seven item
  editors. The old code saved the Text frame's overlay into
  `bar_config.text_overlay` for Static items too, but the renderer reads
  `static_config.text_overlay`, so the static schema edits the overlay that
  is drawn.
* Deleted: graph, speedometer, core bars, indicator and static widgets,
  `TextLineConfigWidget`, `TextOverlayConfigWidget`, and `src/ui/config/`
  (`LazyConfigWidget`, `ConfigWidget`), their last users.
* Behaviour: per-widget Copy/Paste of the graph and speedometer configs is
  the Display page's type-tagged Copy/Paste; the font Copy/Paste buttons of
  core bars and speedometer tick labels are gone; a family-only pick in the
  font dialog keeps the current size; the in-widget previews are gone (the
  live panel is the preview).

Next: the ten themed combo frames (`sections()` from their Frame/Headers/
Layout/… pages, the Theme page staying as the Theme node's editor), then
the two clock displayers and css_template.

## 18. Phase 4a as shipped (2026-11-06)

The combo frame editors start moving to schemas; synthwave is the first.

* `src/ui/config_tree/schema_combo.rs`: `SchemaComboEditor` implements both
  `DisplayEditor` and `ComboDisplayEditor` over the frame's JSON object
  (`frame_from_wrapper` / `wrapper_from_frame` copy the wrapper's two
  animation keys, as `to_frame`/`from_frame` do). Sections and list items
  are tree nodes; `take_pages()` returns nothing and `ensure_display_editor`
  falls back to `sections()`. Group weights are written silently into the
  model; `transferable`/`apply_transferable` read and write the shared keys
  (`split_orientation` or LCARS's `layout_orientation`, tested); `reorder`
  goes store → `DisplayerConfig::swap_groups/swap_items` → load.
* `src/ui/config_tree/combo_items.rs`: the per-item page over
  `content_items[slot]` — display type, height, and the seven per-type
  schema editors, all reloaded on a type switch.
* `PropKind::EnumList { options, count_key, default_key, item_label }` for
  per-group item orientations (one row per group, count from
  `group_count`, default from the layout orientation);
  `PropKind::List.count_key` keeps an integer beside a list equal to its
  length (LCARS segments).
* Theme node: the generic editor (`GlobalThemeWidget`) serves every panel
  whose display editor has no Theme page, now including schema combos, with
  a Preset dropdown: `ComboThemeConfig::palette_presets(displayer)` (the
  Art Deco / Art Nouveau / Steampunk palettes moved out of their widgets)
  then the built-ins. `inline_payload` prefers that editor; a widget-based
  combo editor still holds the Inline payload in its frame theme.
* Behaviour: picking a synthwave colour scheme no longer rewrites the theme
  colours (the old Theme tab did; the renderer reads the scheme itself and
  the Theme node's presets cover palettes); the per-group orientation
  switches became rows in the Layout section; the widget's preview is gone.
* Retro Terminal and Fighter HUD (2026-11-11): the Screen › Phosphor and
  HUD › HUD colour rows are gone. Their pre-schema widgets wrote the four
  theme colours whenever the preset changed (2993d0b, `to_theme_colors`) and
  Phase 4b kept only the preset write, so the header, dividers and glow
  followed the preset while every content item followed the theme. Both
  frames now render their chrome from `theme.color1` (dim variants derived
  in the renderer), so they match the items and follow a group or global
  theme; the presets became Theme-node palettes
  (`ComboThemeConfig::palette_presets`, from `PhosphorColor::palette` /
  `HudColorPreset::palette`); a saved non-default `phosphor_color` /
  `hud_color` is folded into an Inline theme once on load
  (`settings::adopt_frame_color_preset`, skipped when the old widget already
  synced the colours) and is not written when default. Synthwave is
  untouched: its scheme carries more colours than the theme does.

Next: the other nine frames (same recipe; LCARS adds a segments list with
`count_key`, header and divider objects; `hud_color` and `phosphor_color`
need field editors like `synthwave_scheme`), then delete
`combo_config_base.rs` and the theme-row helpers with the last widget, then
the clocks and css_template.

## 19. Phase 4b as shipped (2026-11-06)

All ten themed combo frames are schema-driven; the hand-written display
widgets left are the two clocks and css_template.

* Nine more `Schema` impls (`crates/types/src/display_configs/*.rs`), each
  with a fit test and a JSON round trip. Common shape: Frame / Background /
  Header / Layout / Animation sections, `group_item_orientations` as an
  `EnumList` over `group_count` defaulting to the layout orientation, header
  fonts as `Font` (`PlainFont` for Industrial, whose header font is a plain
  family + size). LCARS: `segments` is a `List` with `count_key:
  "segment_count"`, `top_header`/`bottom_header` and `divider_config` are
  nested objects, `layout_orientation` is its orientation key.
* Field editors: `hud_color` and `phosphor_color` shared a
  `tagged_color_editor` (unit presets plus `{"custom": Color}`) until both
  rows were removed (2026-11-11, see below); `synthwave_scheme_editor` is
  the surviving editor of that shape.
* Deleted: the nine widgets, `src/ui/combo_config_base.rs` (the typed item
  page, layout/theme/animation page helpers), and `ConfigWidgetBuilder` with
  its two extension traits in `widget_builder.rs` (only the themed widgets
  used them). `editor_registry.rs` lost the `combo_editor!` macro and
  `detach_notebook_pages`; `take_pages` is now always empty and the tree
  reads `sections()` for every displayer.
* Not reproduced: LCARS's "sync segments to groups" checkbox sensitivity
  (it was enabled only when the counts matched; the renderer still applies
  the rule itself), Material's unexposed `group_accent_colors` /
  `group_headers` (the widget never edited them either), the per-widget
  preview canvases.

Next: `clock_analog` and `clock_digital` (polled today; `AnalogClockConfig`
has 24 `ColorSource`s) and `css_template` (its placeholder mapping needs the
slot summaries; probably stays a widget), then the `Section` node for
displayer sub-sections can retire `DisplaySection`.

## 20. Phase 4c as shipped (2026-11-06)

The two clocks are schema-driven; `css_template` is the only hand-written
display widget left, and the test source widget the only polled editor.

* `AnalogClockConfig::props()` (`display_configs/clock.rs`): Face (the
  `background` field editor over `face_background`, border), Numbers,
  Ticks, Hands, Icon (the `text_position` editor for `icon_position`,
  `PlainFont` for `icon_font`/`icon_size`). All 24 `ColorSource`s are
  `Color` rows and follow the Theme node. Hand styles are per hand (the old
  widget's one dropdown wrote all three) and the hand lengths, never
  editable before, are `Float` fractions. `face_style` lists all five
  variants (Modern and Numbers draw like Classic) instead of the widget's
  None/Minimal/Classic/Roman remap; "None" is now the Show numbers box.
  The deprecated `center_indicator` has no row and survives because the
  editor loads and stores the whole object (tested).
* `number_font` is a `FontSource` whose size is a fraction of the radius
  (0.12). `ThemeFontSelector`'s size spin starts at 6, so a plain `Font`
  row would have clamped it to 6 and stored 6.0 on the next edit (six
  times the radius). It goes through the new `radius_font` field editor
  (`field_editors.rs`), which shows the fraction as a percentage and
  divides on the way back; the scaling helpers are unit-tested.
* `DigitalClockConfig::props()` (`display_configs/digital_clock.rs`): Style,
  Time, Date, Timer & alarm, Icon; the three family+size pairs are
  `PlainFont`, the four plain colours `Rgba`. `vertical_layout` has no row
  (neither the widget nor the renderer read it).
* Deleted `clock_analog_config_widget.rs` and `clock_digital_config_widget.rs`
  with their registry adapters. The analog `icon_size` is a percentage (5–30)
  behind a `PlainFont` spin whose tooltip says points; the row's help text
  says what it is.

Next: a decision on `css_template` (its placeholder mapping needs the slot
summaries; it may rightly stay a widget), then retire `DisplaySection` /
`take_pages` in favour of `Section` paths.

## 21. Phase 4d as shipped (2026-11-06)

Structural cleanup, and the css_template decision.

* `NodeKind::DisplaySection(String)` is gone; `Section(Vec<String>)` is the
  one node kind for every page a display editor exposes through
  `sections()`: a section is a one-part path, a list item a longer one.
  `State` keeps them in one `display_pages: Vec<EditorPage>`;
  `resolve_page_node(path, pages)` returns the longest existing prefix as a
  `Section`, else Display (tests cover a section, the empty path, an item
  under Display, an item under its section, and a removed item).
  `rebuild_tree` files every page under `page_node(parent path)`, so
  sections land under Display and items under their section, in
  `sections()` order. `refresh_display_pages` evicts exactly the pages with
  no `(path, widget)` match in the new list: a section keeps its stack
  entry, rebuilt item pages leave.
* `ComboDisplayEditor::take_pages` and `State::theme_page` are gone (no
  editor has a detached Theme tab; the Theme node always builds the generic
  editor). `build_section_page` is gone too: sections go through
  `build_subpage`, so they now sit under a heading with page margins like
  every other page (visible change).
* Decision: `css_template` stays a widget. Its editor is file dialogs,
  placeholder scanning of the HTML, grouped mappings and auto-configuration
  from the slot summaries; a schema route would move those 1,600 lines
  behind a field editor, not delete them. It is the only hand-written
  display widget, and `set_source_summaries` stays on the trait for it.

Remaining: docs consolidation (§8-§21 are a build log; the design sections
above still describe `DisplaySection`), and the open policy on
`ThemeLibrary::resolve_missing_named`.

## 22. Combo items as subtrees (2026-11-06)

User request: "take the combo config to more branches (currently stopping
at each Item)". An item node used to be one long page: caption and order,
a Source expander with the source's editor, a Display expander with the
display-as dropdown, the height controls and all seven per-type schema
editors (built eagerly, only one visible). Now an item has children, as a
standalone panel has Source and Display › sections:

```
Content
└ Group 2                      3 items
  └ Item 2                     CPU → Bar
    ├ Source                   CPU             (only when the source has an editor)
    ├ Style
    ├ Foreground
    ├ Background
    ├ Border
    ├ Text overlay
    │ ├ Line 1 · value
    │ └ Line 2 · unit
    └ Animation
```

* **Item page** keeps the identity of the item: caption and order, the
  Source dropdown (with a hint pointing at the Source node), the display-as
  dropdown and the height controls (the head widget of
  `combo_items::ItemDisplayEditor`). A display type with a single section
  (Text, i.e. `TextOverlayConfig`) keeps its rows on the item page and its
  lines become direct children, exactly as the standalone text displayer
  does under Display (`schema::is_sectioned`, shared with
  `SchemaDisplayEditor::create`).
* **`NodeKind::ItemSource(slot)`** (`itemsrc/<slot>`): heading, Copy/Paste
  Source Config, the source's editor. **`NodeKind::ItemSection(slot, path)`**
  (`item/<slot>/<path>`): a section or a list item of the item's display
  editor, resolved like `Section` through `state::resolve_page_prefix` →
  `item_node_for` (tested).
* **The tree never builds an item editor.** Every slot's children are
  derived from the frame JSON at each `rebuild_tree`:
  `schema::tree_pages(props, value, sectioned)` (types crate, pure, tested
  on bar and text overlay) lists the sections then every list item at
  `[section?, key, index]` — object and variant keys are not path segments,
  lists inside list items are not pages — with live titles
  (`list_item_title`) and search terms (`collect_search_terms`, both moved
  from `schema_editor.rs` so the editor and the derivation cannot drift).
  `combo_items::item_tree_pages(frame, slot)` picks the sub-config of the
  slot's `display_as` (default item for an unconfigured slot) and
  `ComboDisplayEditor::item_pages` exposes it. Opening a 29-item panel costs
  29 props walks, no widgets.
* **`State::ensure_item_editor(slot)`** builds the source editor and the
  `ItemDisplayEditor` on first use by *any* of the item's pages (a search
  hit may open a section first); `build_data` and `poll` see the source
  editor as before. The display editor builds only the current type's
  `SchemaEditor::for_tree`, dropping it on a type switch and rebuilding
  from the model (bar and text both edit `bar_config`; the old eager seven
  needed a reload-all for that). `refresh_item_pages(slot, select)` mirrors
  `refresh_display_pages`: evict pages with no `(path, widget)` match,
  adopt, rebuild, select. `drop_item_pages` removes every stack page under
  Content by id prefix (`item/`, `itemsrc/`, `group/`, `content`) and
  cleans the editors; the `item_theme_refreshers`/`item_cleanup` lists are
  gone (the dialog iterates `item_editors`).
* **Folded items.** `TreeListModel` autoexpands everything; 29 items with
  eight children each would bury the overview, so `rebuild_tree` folds
  every item row except the ancestors of the reselected node
  (`collapse_items`, skipped while the search filter is active), selecting
  an item unfolds it (`show_selected`), and `select_node` unfolds the way
  down to a hidden target before selecting (a line added under a folded
  item is still selected).
* **Paste Display Config on a combo** now drops the item editors and
  rebuilds the tree: they held copies of the old content items and their
  next edit would have overwritten the pasted sub-config (pre-existing;
  visible now that the tree shows the pasted structure).
* Visible changes besides the subtree: Copy/Paste Source Config moved from
  the item page to the item's Source node; the Content hint and the Display
  hint were reworded; "Item height (px)" and "Display as" rows sit directly
  under the Source dropdown.

Remaining as in §21: docs consolidation and the `resolve_missing_named`
policy.

## 23. Clock items and short field ids (2026-11-07)

User request: "when setting an Item source as clock, the Display as drop
down should show Text/Analog and then show the appropriate config options",
and, mid-task, "on all text fields in the combo, when I change the field in
the field drop down (Group>Item>Line) the panel stops rendering the text".

* **Display types per source.** `combo_items::types_for_source(source_id)`
  is the "Display as" list: a `clock` slot offers Text and Analog, every
  other source the eight metric types. Analog is a new
  `ContentDisplayType::ClockAnalog` (`"clock_analog"`) with
  `ContentItemConfig::clock_analog_config: AnalogClockConfig`; the tree
  files its Face, Numbers, Ticks and Hands sections under the item through
  the usual `Sub` (`clock_item_props` drops the Icon section — only the
  standalone clock draws the alarm/timer icon and owns the click that opens
  the alarm dialog). `combo_displayer_base::render_content_clock` draws the
  face in both content paths (generic and LCARS) from the slot's
  `hour`/`minute`/`second`/`millisecond` via `combo_utils::clock_hands`,
  checked headlessly with `art_deco_preview` on a clock slot. The second
  hand moves at the combo's `update_interval_ms`, so "Sweep the second
  hand" only sweeps when that interval is short.
* **The dropdown row always equals the model.** GTK fires nothing when the
  shown row is re-selected, so a stored type the source does not offer
  would be unreachable. `coerce_display_as(model, slot, source_id)` writes
  the first offered type when the stored one is not offered (bar → text on
  a clock slot, clock_analog → bar on a metric slot); the dialog runs it on
  load (`State::reset_from` → `coerce_item_displays`, committed like an
  edit when anything changed), on a source switch (`set_item_source`,
  before `structure_changed` rebuilds the subtree) and once more as a
  safety net in `ItemDisplayEditor::new`. The item subtitle shows the
  dropdown label (`display_type_label`) instead of the enum's debug name.
* **Blank text after picking a field.** Item editors were handed
  `slot_fields()` — the combo-wide ids (`group1_1_time`) the frame-level
  editor uses — but an item renders against `get_slot_values()`, the slot's
  values with the prefix stripped, so every id the picker wrote was one the
  renderer never finds. `util::item_fields(source_id)` now gives item
  editors the renderer's own keys (caption, value, unit, numerical_value)
  followed by the source's short ids, matching what the classic widget did
  before Phase 4b. `PanelData::sync_group_structure` additionally runs
  `DisplayerConfig::strip_item_field_prefixes`
  (`combo::strip_slot_prefixes`) so a config saved with the long form by
  v0.9.0–0.9.2 heals at apply and at panel creation.

Remaining as in §21: docs consolidation and the `resolve_missing_named`
policy.

## 24. Group title bars, Item copy/paste, arc fill (2026-11-07)

User request: "Groups in Combo panels should have an optional, configurable
title bar. The Copy/Paste style buttons are all broken - may I suggest we
make it an Item copy/paste button. The Arc Colours section should give arc
fill options (solid/gradient - and the gradient bar should just show the
horizontal gradient, not the circle it currently shows."

* **Group title bars.** `GroupTitleConfig` (`crates/types/src/combo.rs`:
  enabled, text, themed font, bold, themed colour, alignment, height, an
  optional filled bar) lives in every frame as `group_titles`, parallel to
  the group's size weight and item orientation (swapped with them by
  `swap_groups`, carried across frame types by `TransferableComboConfig`).
  It is display state because its font and colours resolve against the
  frame's theme; the source's `GroupConfig` stays item count and weight.
  `render_group_title` (render crate) draws the bar inside the group's
  rectangle and returns the height used; the generic path and LCARS hand
  the rest to the items, so `calculate_group_layouts`, dividers and the
  layout cache are untouched. Material's own `group_headers` turned out to
  have no caller; left as is. Edited on the Group page: a "Title bar"
  `SchemaEditor` over `GroupTitleConfig::props()` bound through
  `ComboDisplayEditor::{group_title, set_group_title}` (the frame's list is
  padded with defaults), kept in `State::group_editors`, dropped with the
  Content pages and themed by `propagate_theme`.
* **Copy Item / Paste Item.** The item page carries the item as a whole:
  `ComboItemClip` (source id, caption override, flat source config and the
  `ContentItemConfig` JSON). Paste writes the slot into the working combo
  source, the item into the frame (`ComboDisplayEditor::set_content_item`),
  coerces the display type to one the pasted source offers and runs
  `structure_changed`, which drops the item's editors (they held the old
  item), reloads the frame with the synced structure, rebuilds the subtree
  and commits. The item Source node's Copy/Paste Source Config is gone (the
  item pair subsumes it); the panel-level Source/Display/Style pairs stay.
  Every pasted displayer config now goes through
  `State::load_display_config`: group sync and field-prefix strip for a
  combo, this panel's resolved theme, and the item editors rebuilt (Paste
  Style used to load another panel's frame without any of that). The JSON
  forms the buttons exchange are covered by two round-trip tests over every
  displayer and source id (`panel.rs`).
* **Arc fill.** `ArcDisplayConfig::fill` (`ArcFill::{Solid, Gradient}`,
  default Gradient so saved arcs are unchanged) plus `solid_color`; the
  Colours section shows Fill, then either the colour or the stops with their
  transition and mode. The stops row is `linear: true` now, so the preview is
  the horizontal ramp. `render_arc` resolves a solid fill as the same colour
  at both ends, so every lookup downstream is a constant. The speedometer
  still uses the sweep preview.

Remaining as in §21: docs consolidation and the `resolve_missing_named`
policy.

## 25. Analog clock tweaks (2026-11-07)

User request: an outline show/hide on the Face page, the Numerals dropdown
without the two options that changed nothing, and a radius for where the
numerals sit. One config (`AnalogClockConfig`) and one renderer
(`render_analog_clock_with_theme`) serve both the standalone clock and the
combo item, so the change reaches both:

* `show_border` (default true) gates the outline colour and width rows and
  the outline stroke.
* `FaceStyle::{Modern, Numbers}` are gone; `classic` carries them as serde
  aliases so saved configs load as Classic (tested).
* `number_radius_percent` (default 75, the old constant) places the numerals
  as a percentage of the radius, shown under Numbers.

The speedometer's Track section got the arc's treatment the same day:
`track_fill` (`TrackFill::{Solid, Gradient}`, default Gradient) chooses
between the track colour and the colour zones, whose stops row is now
`linear: true` (horizontal preview). A gradient with a single zone still
draws solid, as before, so no saved speedometer changes.
Its tick labels take a themed font now (`TickLabelConfig::font_source`,
`Option<FontSource>` over the legacy `font_family`/`font_size` the way text
lines do; `resolved_font` falls back to the plain pair, tested), and the one
Ticks section is three: Major ticks, Minor ticks, Tick labels, so each is
its own tree node.

## 26. Combo item intervals, level bars, core bars labels (2026-11-07)

User report, three parts: the Level Bar item type drew nothing in combo
panels; the Core Bars Labels page offered a plain font where the text
overlay offers theme fonts; and core bars items ignored the update interval
set on their Source page. Only the Content page's interval changed anything,
and it overrode every item's.

* **Item intervals.** Each slot's source config carries an
  `update_interval_ms` (its Source page shows the row), but
  `ComboSource::update` refreshed every child on every poll and the scheduler
  polled the combo at the Content interval alone. Now
  `ComboSourceConfig::poll_interval_ms()` is the fastest of the Content
  interval and every slot's own interval, and `SourceConfig::update_interval_ms()`
  returns it for combos, so the scheduler, the shared-source manager and the
  config hash all see it; `ComboSource::update` keeps `child_last_update` and
  skips a child whose own interval has not passed (10 ms tolerance for poll
  jitter), so a slot with no interval follows every poll, a slower one waits,
  a faster one speeds the poll up. The update loop's base tick dropped from
  250 ms to 100 ms, the dialogs' minimum; sysinfo's 200 ms CPU floor
  (`MINIMUM_CPU_UPDATE_INTERVAL`) remains. The Content page hint says so.
* **Level Bar.** Both content paths had a stub ("render as text, level bar
  can be added later"), so an item showed only its text overlay. It now goes
  through `render_content_bar` at the item's fixed height (the layout already
  sized it like a graph); picking Level Bar in the dropdown sets the bar's
  style to segmented, the look of gSens's level bar, and the Bar page can
  change it (`write_display_as`, tested).
* **Core Bars.** `label_font_source: Option<FontSource>` over the legacy
  `label_font`/`label_size` (themed Font row with legacy fallback, like tick
  labels; `resolved_label_font`, tested), resolved once per draw in
  `render_core_bars_with_values` and threaded to the label calls. The item's
  `animate` switch is honoured on both combo paths (`update_core_bars` ANDs it
  with the frame's), its speed row is hidden for combo items
  (`core_bars_item_props`) because the shared tick runs at the frame's speed,
  and the standalone's speed range is 0.5–30 per second like the frames' (it
  was 0.01–1.0 around a default of 8, so opening the page clamped the default
  and the bars crawled). Two pipeline tests in `combo_displayer_base.rs` pin
  that a core bars item's targets follow each reading and snap when its
  animation is off.

## 27. Item orientation on the Group page (2026-11-07)

User request: the per-group item orientation (vertical/horizontal) sat
under Display › Layout as an `EnumList` row per group; it belongs on each
Group page under the item count. The frame's `group_item_orientations` list
is unchanged (parallel to the weights, swapped by `swap_groups`, carried by
the transfer); the Group page now shows an "Item layout" dropdown fed by
`ComboDisplayEditor::{group_orientation,set_group_orientation}`
(`schema_combo::group_orientation_of` / `set_group_orientation_in`: the
group's own entry, else the frame's group layout; a write pads the list with
the layout orientation up to the group, as the old row did; tested). The
dropdown re-reads the value on `map`, so a "Group layout" change made while
the page was hidden shows when it is next selected (Revert and structure
changes rebuild the page outright). The
`EnumList` rows are gone from all ten frame files' Layout sections; "Group
layout" (`split_orientation`, LCARS `layout_orientation`) stays there as the
default for groups without an entry. `PropKind::EnumList` itself remains
for any future list.
