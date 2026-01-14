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
2. **Set up the combo source** - Add sources (CPU, GPU, Memory, etc.) in the panel's source configuration
3. **Select your template** - Point to your HTML file in the Template tab
4. **Configure mappings** - Map placeholders to data sources in the Mappings tab

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

Templates can include metadata that enables the "Auto-configure" button to automatically set up mappings.

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
| `source` | Yes* | Source type: `cpu`, `gpu`, `memory`, `disk`, `clock`, `network` |
| `instance` | No | Instance index (1-based for specific slot, 0 for auto-assign) |
| `field` | Yes* | Field to use: `value`, `caption`, `unit`, `max`, `percent`, `time` |
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

## File Locations

- **User templates**: Store in `~/.config/rg-sens/templates/`
- **System examples**: `/usr/share/rg-sens/examples/` (if installed via package)

## Available Source Types

| Source | Fields | Description |
|--------|--------|-------------|
| `clock` | `time`, `hour_value`, `minute_value`, `second_value` | Current time |
| `cpu` | `value`, `caption`, `unit`, `max`, `percent` | CPU metrics |
| `gpu` | `value`, `caption`, `unit`, `max`, `percent` | GPU metrics |
| `memory` | `value`, `caption`, `unit`, `max`, `percent` | RAM usage |
| `disk` | `value`, `caption`, `unit`, `max`, `percent` | Disk usage |
| `network` | `value`, `caption`, `unit`, `download_speed`, `upload_speed`, `total_download`, `total_upload` | Network interface stats |

## See Also

- [Art Nouveau Panel Example](../examples/art_nouveau_panel.html) - Full-featured example with gauges
- [CSS Template Example](../examples/css_template_example.html) - Basic template structure
