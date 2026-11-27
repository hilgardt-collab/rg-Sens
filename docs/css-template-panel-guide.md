# CSS Template Panel Guide

This guide explains how to create custom CSS Template panels for rg-Sens. CSS Template panels let you design fully custom visualizations using HTML, CSS, and JavaScript.

## Overview

CSS Template panels use a WebView to render HTML/CSS content, with placeholder values that get replaced with real-time system data. This gives you complete control over the visual design.

**Key features:**
- Full HTML5/CSS3/JavaScript support
- Real-time data updates via placeholders
- Auto-configuration from template metadata
- Hot-reload during development

## Quick Start

1. **Create a new panel** - In rg-Sens, add a new panel and select "CSS Template" as the displayer type
2. **Select your template** - Point to your HTML file in the Template tab (its stylesheet is linked from the HTML, or set it as the CSS file)
3. **Scan Placeholders** - Adds the sources the template asks for under Content and maps each placeholder to its slot
4. **Adjust** - Pick sensors and their settings on the Bindings tab; set the template's own options (palette, layout, frame rate) on the Display tab

## Creating a Template

### Basic Structure

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <link rel="stylesheet" href="my-panel.css">
</head>
<body>
    <div class="metric">
        <span class="label">{{0}}</span>
        <span class="value">{{1}}</span>
        <span class="unit">{{2}}</span>
    </div>
</body>
</html>
```

### Placeholders

Placeholders use the format `{{N}}` where N is a number starting from 0:
- `{{0}}` - First placeholder
- `{{1}}` - Second placeholder
- etc.

Each placeholder maps to a field from your configured data sources.

### Standard Field Pattern

Most metrics use 4 fields per source:

| Field | Purpose | Example |
|-------|---------|---------|
| `caption` | Display label | "CPU Temperature" |
| `value` | Numeric value | "65" |
| `unit` | Unit string | "°C" |
| `max` | Maximum value | "100" |

So a typical mapping pattern is:
- `{{0}}` = Source 1 caption
- `{{1}}` = Source 1 value
- `{{2}}` = Source 1 unit
- `{{3}}` = Source 1 max
- `{{4}}` = Source 2 caption
- ... and so on

## Auto-Configuration

Templates can include metadata that lets **Scan Placeholders** set the whole panel up: it adds the source slots the template asks for to the panel's Combination source (under Content; slots you already have are kept and reused), then maps every placeholder to the slot with the matching source and metric. **Auto-configure** on the Mappings tab only maps: it fills in placeholders that have no source yet and leaves the rest alone.

### Adding Auto-Config Metadata

Add a JSON script block to your HTML:

```html
<script type="application/json" id="rg-placeholder-config">
{
    "0": {
        "hint": "CPU Temperature - Caption",
        "source": "cpu",
        "instance": 1,
        "field": "caption"
    },
    "1": {
        "hint": "CPU Temperature - Value",
        "source": "cpu",
        "instance": 1,
        "field": "value",
        "format": "{:.0}"
    },
    "2": {
        "hint": "CPU Temperature - Unit",
        "source": "cpu",
        "instance": 1,
        "field": "unit"
    },
    "3": {
        "hint": "CPU Temperature - Max",
        "source": "cpu",
        "instance": 1,
        "field": "max"
    }
}
</script>
```

### Config Properties

| Property | Required | Description |
|----------|----------|-------------|
| `hint` | No | Human-readable description shown in UI |
| `source` | Yes* | Source type, the first word of the source's name in lower case: `cpu`, `gpu`, `memory`, `disk`, `clock`, `network`, `fan`, `system`, `uptime` |
| `metric` | No | What the slot must measure, as the source's own field name in snake_case: `temperature`, `usage`, `frequency` (cpu); `temperature`, `utilization`, `clock_core`, `power_usage`, `memory_used` (gpu); `used` (memory, disk); `download_speed`, `upload_speed` (network). Without it any slot of the source matches |
| `instance` | No | `0` takes the next matching slot no other placeholder has taken; `N` takes the Nth matching slot, so several placeholders can read one slot (the clock's `time`, `day_name` and `month_name` all use `1`) |
| `field` | Yes* | Field to use: `value`, `caption`, `unit`, `max`, `min`, `percent`, `time`, or any other field the source publishes (`max`/`min` are the slot's limits) |
| `format` | No | Format string, e.g., `"{:.1}"` for 1 decimal place |

*Required for auto-configure to work

## JavaScript Integration

### Custom Update Handler

Templates can define a custom `updateValues` function for advanced visualizations:

```html
<script>
window.updateValues = function(values) {
    // values is an object: { "0": "65", "1": "CPU", ... }

    // Example: Update a progress bar
    const percent = parseFloat(values['1']) / parseFloat(values['3']) * 100;
    document.getElementById('progress').style.width = percent + '%';

    // Example: Change color based on value
    const temp = parseFloat(values['1']);
    const bar = document.getElementById('temp-bar');
    if (temp > 80) {
        bar.style.backgroundColor = 'red';
    } else if (temp > 60) {
        bar.style.backgroundColor = 'orange';
    } else {
        bar.style.backgroundColor = 'green';
    }
};
</script>
```

### Available JavaScript Functions

rg-Sens injects these functions:

```javascript
// Set animation speed multiplier
window.setAnimationSpeed(1.5);

// The panel's Animation switch and speed, sent with every value update.
// It sets --rg-animation-speed, puts data-rg-motion="on|off" on <html> and
// fires an `rg-motion` event (detail: {enabled, speed}) for templates that animate.
window.rgSetMotion(true, 1.0);

// Set theme colors (from panel configuration)
window.setThemeColors({
    color1: '#ff6b6b',
    color2: '#4ecdc4',
    color3: '#45b7d1',
    color4: '#96ceb4'
});
```

### CSS Custom Properties

These CSS variables are available:

```css
:root {
    --rg-theme-color1: #ff6b6b;
    --rg-theme-color2: #4ecdc4;
    --rg-theme-color3: #45b7d1;
    --rg-theme-color4: #96ceb4;
    --rg-animation-speed: 1;
}
```

## Styling Tips

### Transparent Background

For panels that overlay on desktop backgrounds:

```css
html, body {
    background: transparent;
}
```

### Responsive Design

Use the panel dimensions in your CSS:

```css
.panel {
    width: 480px;  /* Match your panel width */
    height: 1920px; /* Match your panel height */
}
```

### Smooth Animations

Use CSS transitions for smooth value updates:

```css
.value {
    transition: all 0.3s ease-out;
}

.progress-bar {
    transition: width 0.5s ease-out;
}
```

### SVG Gauges

SVG works great for circular gauges:

```html
<svg viewBox="0 0 100 100">
    <circle class="track" cx="50" cy="50" r="40"
            fill="none" stroke="#333" stroke-width="8"/>
    <circle class="fill" cx="50" cy="50" r="40"
            fill="none" stroke="#4ecdc4" stroke-width="8"
            stroke-dasharray="251.3" stroke-dashoffset="125"/>
</svg>
```

```javascript
// Update gauge (0-100%)
function updateGauge(percent) {
    const circumference = 2 * Math.PI * 40; // 251.3
    const offset = circumference * (1 - percent / 100);
    document.querySelector('.fill').style.strokeDashoffset = offset;
}
```

## Example Templates

### Simple Text Display

```html
<!DOCTYPE html>
<html>
<head>
    <style>
        body {
            font-family: sans-serif;
            background: #1a1a2e;
            color: #eee;
            padding: 20px;
        }
        .metric {
            margin: 10px 0;
        }
        .label {
            color: #888;
            font-size: 12px;
        }
        .value {
            font-size: 32px;
            font-weight: bold;
        }
        .unit {
            font-size: 14px;
            color: #666;
        }
    </style>
</head>
<body>
    <div class="metric">
        <div class="label">{{0}}</div>
        <span class="value">{{1}}</span>
        <span class="unit">{{2}}</span>
    </div>
</body>
</html>
```

### Progress Bar

```html
<!DOCTYPE html>
<html>
<head>
    <style>
        .bar-container {
            width: 200px;
            height: 20px;
            background: #333;
            border-radius: 10px;
            overflow: hidden;
        }
        .bar-fill {
            height: 100%;
            background: linear-gradient(90deg, #4ecdc4, #45b7d1);
            transition: width 0.5s ease-out;
        }
    </style>
</head>
<body>
    <div class="label">{{0}}</div>
    <div class="bar-container">
        <div class="bar-fill" id="bar"></div>
    </div>
    <div class="value">{{1}} / {{3}} {{2}}</div>

    <script>
        window.updateValues = function(values) {
            const percent = (parseFloat(values['1']) / parseFloat(values['3'])) * 100;
            document.getElementById('bar').style.width = percent + '%';
        };
    </script>
</body>
</html>
```

## Custom Layouts: Groups and Widgets Chosen in the Dialog

A template normally arranges its own groups. One that also declares its
building blocks lets the user decide how many groups there are and which
widget each sensor is shown with (Panel Properties › Display › Layout ›
"My own groups and widgets"). The groups and items are the ones under
Content, the same as for every other combo panel:

- **Group page:** title, the group style when the template has more than
  one, and a board: the group's grid of square cells, where a small widget
  takes 1 x 1, a medium one 2 x 2 and a large one 4 x 4. Drag a box to any
  free cells; drag its right or bottom edge, or the marked corner, to give
  it any other block of cells (a bar at 4 x 1, a column at 1 x 3). A group
  shows at most 6 widgets.
- **Item page:** the source as always, plus Widget, Size and Colour.

The grid is fitted to the room the group has and every widget is scaled to
its cells, so any widget can be shown in any size and any placement fits
(four small ones beside a large one is a 5 x 4 grid). A widget in a block
the user shaped is drawn from whichever of its kind's prototypes fills that
block best, so give a kind prototypes of different proportions (a compact
bar, a long one) and state each one's `data-rg-width` / `data-rg-height`.

The Bindings tab is greyed out while a custom layout is on: every sensor is
bound by its widget. What the template shows outside its groups keeps the
binding it had.

Left on "The template's own layout", or with a template that declares no
prototypes, nothing changes: the page is loaded exactly as written.

### What the template declares

```html
<meta name="rg-group-size" content="480x345+60">   <!-- room the tallest group needs, + what sits outside the groups -->

<div class="groups">
<!--rg-groups 0-71 90-->       <!-- the template's own groups; the placeholders they use -->
  <section>...</section>
<!--/rg-groups-->
</div>

<template data-rg-group="section" data-rg-label="Section">
  <section class="section"><h2>{{title}}</h2><!--rg-items--></section>
</template>

<template data-rg-widget="dial" data-rg-label="Dial" data-rg-size="large">
  <svg data-gauge="{{value}}" data-max="{{max}}">...</svg>
  <span data-cap="{{caption}}"></span>
</template>
```

| Declaration | Meaning |
|---|---|
| `<!--rg-groups N-M ...-->` … `<!--/rg-groups-->` | The region a custom layout replaces. The numbers are the placeholders used inside it; their rows leave the Bindings tab while a custom layout is on. |
| `<template data-rg-group="id">` | A group shell. `{{title}}` is the group's title, `<!--rg-items-->` where its widgets go. `{{index}}` counts the groups drawn from 1 and `{{parity}}` is `odd` or `even`, for designs whose groups alternate. The first shell is the default. A shell is laid out at the group's real size and is not scaled: put fixed-size artwork in a widget. |
| `<template data-rg-widget="kind" data-rg-size="large\|medium\|small">` | A widget, as drawn for that size. One drawing is enough: it is scaled to whatever size the user picks. Further drawings of the same kind are used for the sizes nearest theirs; the first one is the size the kind starts in. |
| `data-rg-width="330" data-rg-height="190"` | The size the widget is drawn at, in design pixels. It is laid out at exactly that size and scaled, whole, to its cells. |
| `data-rg-label` | The name shown in the dialog. |
| `data-rg-sources="fan clock"` | The sources the widget is made for: offered first for those, not at all for others. A clock is offered only widgets made for it. |
| `data-rg-default-for="memory disk"` | The sources the widget is the first choice for in a fresh layout, while it stays on offer for every source. |
| `data-rg-group="poster"` | The group shell a group gets when this is its first widget (and the group's style is Automatic). |
| `data-rg-colors="red yellow blue black"` | The colours the widget can be drawn in. `{{color}}` in its markup becomes the chosen one (usually a class name of the template). Left on Automatic, the colours take turns through the group, in the order listed. |
| `rg-group-size` | `WxH+E`: the room one group gets and the extra height outside the groups. `rg-layouts` is recomputed from it for the number of groups. |

Inside a widget, `{{field}}` is replaced by a placeholder number of its own
(1000 and up), bound to that field of the widget's sensor: `{{value}}`,
`{{caption}}`, `{{unit}}`, `{{max}}`, `{{min}}`, or any field the source
publishes (`{{time}}`, `{{day_name}}`). It is a bare number, so it goes
where a hand-written template puts one (`data-bar="{{value}}"`); wrap it in
braces again for text (`{{{{value:.1}}}}`). `{{value:.1}}` asks for one
decimal; `{{slot}}` is the sensor's slot name, for ids. The script must read
ids from attributes, never from constants (`v['65']`), and must not assume
that a unit is the value's number plus one.

rg-Sens builds each group's widgets as

```html
<div class="rg-items" style="grid-template-columns:repeat(5,...);grid-template-rows:repeat(4,...)">
  <div class="rg-item rg-size-large rg-widget-dial" style="grid-column:2/span 4;grid-row:1/span 4">
    <div class="rg-fit" style="width:330px;height:190px"> the prototype </div>
  </div>
</div>
```

and the bridge script scales each `.rg-fit` to its `.rg-item`. The rules for
these classes have no specificity (`:where`), so the template's stylesheet
can restyle them. The grid must get a definite box from the group shell, or
its cells collapse to their content: give the shell's flex or grid ancestors
`min-height: 0`, and equal, clipped rows to the groups themselves:

```css
.groups:has(.rg-items) { grid-auto-rows: minmax(0, 1fr); }
.section:has(.rg-items), .section:has(.rg-items) > div { min-height: 0; overflow: hidden; }
```

A fresh custom layout gives the first item of each group, and every
temperature, the template's first general widget kind at medium (2 x 2), and
the other items its second kind as 2 x 1 blocks, which stack beside it; an
item whose source has a widget made for it gets that widget. So declare the
showpiece (a dial) first and the bar second, and draw them for those boxes.

Every item's Widget list ends with "Not shown": a sensor that only feeds
something outside the groups (a footer's uptime) is drawn in no group, which
is also its default; a group with nothing to draw is left out.
`examples/bauhaus_panel.html` is the worked example; every example panel
but Steampunk (one composed picture) declares prototypes.

To look at a layout without the app:

```bash
cargo run --example webkit_leak -- template=examples/bauhaus_panel.html \
    layout=layout.json feed=values.json size=480x1500 snapshot=out.png duration=8
```

## Troubleshooting

### Placeholders not updating
- Check that mappings are configured in the Mappings tab
- Verify the source is providing data (check other panel types)
- Open browser dev tools if available to check for JavaScript errors

### Template not loading
- Verify the HTML file path is correct
- Check file permissions
- Try a simple template first to isolate issues

### Styles not applying
- External CSS files must be in the same directory or use absolute paths
- Check for CSS syntax errors
- Verify the CSS file path in the Template tab

### Hot-reload not working
- Ensure "Hot reload" is enabled in the Template tab
- Some changes may require panel restart

## Known Limitations

### Template Options

A template can offer settings (a palette, a frame rate, a full-scale value)
that appear on the panel's **Display** tab under *Template Options*. Declare
them in a JSON block:

```html
<script type="application/json" id="rg-template-options">
[
  {"id": "palette", "label": "Palette", "type": "choice", "default": "cyan",
   "choices": [
     {"value": "cyan", "label": "Cyan + Magenta", "set": {"--a": "#22E4FF", "--b": "#FF2BD6"}},
     {"value": "lime", "label": "Lime + Cyan",    "set": {"--a": "#B6FF3B", "--b": "#22E4FF"}}]},
  {"id": "scheme", "label": "Scheme", "type": "choice", "var": "--scheme", "default": "classic",
   "choices": [{"value": "classic", "label": "Classic"}, {"value": "alert", "label": "Alert"}]},
  {"id": "fps", "label": "Frames per second", "type": "number", "var": "--motion-fps",
   "min": 0, "max": 30, "step": 1, "default": 6, "help": "0 turns motion off"},
  {"id": "glow", "label": "Glow", "type": "color", "var": "--glow", "default": "#FF7A1A"},
  {"id": "alert", "label": "Alert when hot", "type": "toggle", "var": "--auto-alert", "default": true}
]
</script>
```

| Type | Control | Written to the page |
|------|---------|---------------------|
| `choice` | dropdown | `var: value`, and every property in the choice's `set` |
| `number` | spin button (`min`, `max`, `step`) | `var: number` |
| `color` | colour button | `var: #RRGGBB` |
| `toggle` | switch | `var: 1` or `var: 0` |

The chosen values are saved with the panel and applied as CSS custom
properties on `:root`, in a stylesheet placed after the template's own, so
the template only has to use `var(--name)` (or read it from script with
`getComputedStyle`). An option left at its default writes nothing: make the
`default` the value your stylesheet already has. Changing an option reloads
the page. Only declared properties are written, and only with values the
option accepts.

### Fitting the Panel

A template drawn for one size can be scaled to any panel. State the size:

```html
<meta name="rg-design-size" content="480x1920">
```

rg-Sens then scales the page to the panel and centres it. `--rg-fit` chooses
how: `cover` (the default: fill the panel, cropping what does not fit),
`contain` (everything visible, with empty bands), `stretch` (distorted to the
panel) or `fill` (below). A template without the tag is left exactly as it
is, so a template that is already responsive needs nothing.

#### Re-arranging for the panel's shape

A design can offer other arrangements of itself. List the size it needs with
each number of columns, narrowest first:

```html
<meta name="rg-layouts" content="480x1920 960x1060 1440x775 2880x450">
```

An arrangement's column count is its width over the first one's: one, two,
three and six columns above, which for six groups is 1x6, 2x3, 3x2 and 6x1.
The arrangement that can be shown largest is chosen (or the one with N
columns, when `--rg-columns` is N), and the page is told through:

| On `<html>` | Meaning |
|---|---|
| `--rg-w`, `--rg-h` | the box the design has, in its own pixels |
| `--rg-cols` | the number of columns, as a number for `repeat()` |
| `data-rg-columns` | the same, for selectors |

An `rg-layout` event (`detail: {columns, width, height}`) fires as well.

With `--rg-fit: fill` the box is not the arrangement's listed size but the
panel's own proportions, at least as large as the listed size. Nothing is
cropped, banded or distorted: the page's CSS takes up the slack. The whole
of it is a grid:

```css
.panel  { width: var(--rg-w, 480px); height: var(--rg-h, 1920px);
          display: flex; flex-direction: column; gap: 14px; }
.groups { flex: 1; min-height: 0; display: grid; gap: 14px 22px;
          grid-template-columns: repeat(var(--rg-cols, 1), minmax(0, 1fr));
          align-content: stretch; }           /* rows share the spare height */
.groups > section { display: flex; flex-direction: column; }
.groups > section > .card { flex: 1; }       /* a card fills its cell */
```

Keep a title strip and a footer outside `.groups`, so they span the width.
What to watch:

- **Listed heights.** A grid row is as tall as its tallest group, so each
  arrangement needs its own height. Too small crops; too large only costs a
  little scale. Lay the page out at each column count with its height left
  free and read the height.
- **Full-page artwork** (a wall, a frame, a wallpaper) must cover the box,
  not 480x1920: a CSS background, an SVG `<pattern>`, or `width="100%"
  height="100%"` without a `viewBox`.
- **Things that grow with their column** (a square with `aspect-ratio`)
  make a group taller in a wide cell than the listed height allows; cap
  their width.
- **Fixed-size drawings** (a gauge, a chart) stay their size; centre them.

Every example but Steampunk works this way and declares `fill` as its default
fit, with a Layout option for the column count. How each one's full-page
artwork follows the box is worth copying:

| Artwork | How it follows the box | Examples |
|---|---|---|
| Repeating texture | CSS background or SVG `<pattern>` | LCD, Victorian, Gothic, Mughal Peacock, Western, Japandi (daylight bands), Filigree (side stems: one 80px period, the strip's height rounded down to whole periods), Brutalist (wall boards and tie holes) |
| Scattered pieces | placed by percentage (glows, rain drips), or in a strip down each side margin with percentage heights (bubbles) | Glass & Gumdrop, Brutalist |
| A picture in a frame | an SVG with a `viewBox` and `preserveAspectRatio="xMidYMax meet"`, whose sky and ground rectangles run far past the viewBox: the picture is never cropped and the frame is filled around it | Brutalist (the opening) |
| Rails with evenly spaced beads | CSS backgrounds, the beads with `background-repeat: space` | Filigree |
| Strips down the sides | an SVG per side, `height="100%"`, `preserveAspectRatio="xMinYMin slice"` (cut off, not squeezed), the right one mirrored | Art Deco, Art Nouveau, Gothic, Mughal Peacock |
| Corner pieces | an SVG per corner, placed by CSS | Art Deco, Gothic, Filigree |
| A rim or inlay | a CSS border | Art Nouveau, Leather & Wood |
| A frame with notches | drawn by the script on the `rg-layout` event | Warrior |
| Wood grain | one SVG stretched to the box (`preserveAspectRatio="none"`) | Leather & Wood |

LCARS has three frames, not six groups, so it lists one, two and three
columns and its frames stretch. Filigree's pieces keep their size, because each
frame is a wire drawing with a gap for its title, and sit in the middle of
their cells; Leather & Wood's pads keep their height. Steampunk's background is one composed
picture (plates, a pipe frame, gears and gauges at fixed spots); it lists no
arrangements and only scales. Every example declares a `fit` option for the
Display tab.

### Noise Textures: Bake Them

WebKit recomputes an SVG filter over its whole region every time anything
over or under it repaints. A full-page noise texture (`feTurbulence` on a
100% rectangle: plaster, velvet, paper grain) therefore costs a full render
on every value update: measured with values changing once a second and
motion off, the Western example's web process used about 40% of a core, and
under 2% once its textures were baked.

Mark such an element and rg-Sens draws it once into a bitmap that takes its
place, and again only when its `<svg>` changes size:

```html
<svg width="100%" height="100%" style="position: absolute; top: 0; left: 0">
  <rect data-rg-bake width="100%" height="100%" filter="url(#noise)"></rect>
</svg>
```

Only the element and its filter go into the bitmap, so mark plain shapes
whose look comes from the filter (a fill colour from the stylesheet is
carried over), and keep them the first drawn content of their `<svg>`: the
bitmap is put where the first marked element was. Small filtered shapes
inside a card are not worth marking. Opened in a browser, the attribute does
nothing. The Japandi, Gothic, Western, Filigree and Brutalist examples use it.

### Memory Usage

CSS Template panels render in a separate WebKitGTK web process. Measured with
`cargo run --example webkit_leak` (see the file's header), the growth people saw
is in that process, follows how much the page **repaints**, and is memory WebKit
releases once it considers itself under memory pressure. By default that
threshold is a fraction of system RAM, so on a desktop it never triggered.

rg-Sens now gives the web process a memory limit (192 MB; WebKit starts
releasing caches at a third of it), which keeps a template updating once a
second flat. The periodic WebView reset that used to back this up is off by
default; `RG_SENS_WEBKIT_RECYCLE_EVERY=300` in the environment turns it back
on (a reset every 300 value updates).

**What costs memory in a template:**
- Infinite CSS animations of paint-heavy properties, above all an animated
  `text-shadow` or `box-shadow` blur: a glowing clock in an earlier Art Deco
  example was the whole of its growth. Animating `opacity` or `transform` is cheap.
- Transitions that are restarted faster than they finish.
- JavaScript that keeps references to DOM nodes between updates (the bundled
  `rg-sens-gauges.js` does not).

**What a panel costs:** the first CSS Template panel starts WebKit's web and
network processes, 300-370 MB in total across the three processes on the test
machine, more for a larger panel; nothing is started while no CSS Template
panel exists. CPU depends on how often the page repaints. A template with a
continuously running animation repaints every frame: an earlier Art Deco
example with a pulsing glow used tens of percent of one core. The current
`examples/art_deco_panel.html` has no animations or transitions and writes to
the page only when a value changed; fed synthetic values once a second (about
half of its rows bound) it used about 1% of a core in rg-Sens and under 1% in
the web process.

**Animating on a budget:** every animated frame is repainted in the web process
and presented by rg-Sens, so cost follows the frame rate, not the number of
moving things. The Steampunk example as designed (gears, steam, needles at the
display's 60 fps) used about 0.8 of a core in rg-Sens and 1.0 in the web
process. It ships with its CSS animations paused and stepped from one script
timer (`document.getAnimations()`, `animation.currentTime = ...`) at
`--motion-fps`: about 11% of a core in total at 4 fps, 13% at the default 6,
21% at 8. The panel's Animation switch (`rg-motion` event) and the system's
reduced-motion setting turn it off, which brings it to the same idle cost as
the static panels. Splitting a large SVG into stacked layers
(`will-change: transform`) so that moving parts do not repaint static ones cut
the web process's share by a quarter. Its motion alone holds memory level
(about 165 MB in the web process over 18 minutes); rewriting its glowing
(`text-shadow`) values every second is what grew when this was measured, by
roughly 15 MB a minute even under the memory limit; the periodic WebView reset
was what brought it back then.
The 4 and 8 fps figures were measured before the layer split.

CSS Template panels are enabled by default; start rg-Sens with
`--webkit-disable` to turn them off.

### WebView Limitations

- CSS Template panels spawn a separate WebKitWebProcess for rendering
- Very complex templates may impact system performance
- Some advanced web features are disabled for security (local storage, databases, WebGL)

## File Locations

- **User templates**: Store in `~/.config/rg-sens/templates/`
- **System examples**: `/usr/share/rg-sens/examples/` (if installed via package)

## Available Source Types

| Source | Fields | Description |
|--------|--------|-------------|
| `clock` | `time`, `date`, `day_name`, `month_name`, `day`, `hour`, `minute`, `second` | Current time and date |
| `cpu` | `value`, `caption`, `unit`, `max`, `percent` | CPU metrics |
| `gpu` | `value`, `caption`, `unit`, `max`, `percent` | GPU metrics |
| `memory` | `value`, `caption`, `unit`, `max`, `percent` | RAM usage |
| `disk` | `value`, `caption`, `unit`, `max`, `percent` | Disk usage |
| `network` | `value`, `caption`, `unit`, `download_speed`, `upload_speed`, `total_download`, `total_upload` | Network interface stats |
| `fan` | `value`, `caption`, `unit`, `max` | Fan speed (RPM) |
| `uptime` | `value`, `caption`, `text`, `compact`, `seconds`, `days`, `hours`, `minutes` | Time since boot: `text` is "4 days, 7 hours", `compact` is "4d 07:12" |

## See Also

- [Art Deco](../examples/art_deco_panel.html),
  [Art Nouveau](../examples/art_nouveau_panel.html),
  [Steampunk](../examples/steampunk_panel.html),
  [Neon](../examples/neon_panel.html),
  [LCARS-style](../examples/lcars_panel.html),
  [Warrior](../examples/warrior_panel.html),
  [Leather & Wood](../examples/leather_wood_panel.html),
  [LCD](../examples/lcd_panel.html),
  [Victorian](../examples/victorian_panel.html),
  [Mughal Peacock](../examples/mughal_peacock_panel.html),
  [Bauhaus](../examples/bauhaus_panel.html),
  [Gothic](../examples/gothic_panel.html),
  [Western](../examples/western_panel.html),
  [Japandi](../examples/japandi_panel.html),
  [Glass & Gumdrop](../examples/gumdrop_panel.html),
  [Filigree](../examples/filigree_panel.html) and
  [Brutalist](../examples/brutalist_panel.html) panel examples - full 480x1920
  panels sharing one placeholder mapping: gauges, fan meters, bars, sections
  that hide when unmapped; setup steps are in the comment at the top of each.
  They load their fonts from `examples/fonts/` (SIL Open Font License; Special
  Elite is Apache 2.0), so copy that folder along with a template. The
  Steampunk panel is the animated one: see "Animating on a budget" above
- [CSS Template Example](../examples/css_template_example.html) - Basic template structure
