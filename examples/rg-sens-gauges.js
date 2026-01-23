/**
 * rg-sens-gauges.js
 *
 * A lightweight JavaScript library for CSS template panel visualizations.
 * Handles bars, arcs, polygons, and ring gauges with smooth animations.
 *
 * MEMORY OPTIMIZATION: This version avoids storing persistent DOM references
 * to prevent memory accumulation in WebKitGTK.
 *
 * Usage:
 *   1. Include this script in your HTML template
 *   2. Add data-gauge attributes to your elements
 *   3. The library auto-initializes and exposes window.updateValues()
 *
 * Supported gauge types:
 *   - bar: Linear bar that fills by width/height percentage
 *   - arc: SVG path that fills using stroke-dashoffset
 *   - polygon: SVG polygon that fills using stroke-dashoffset
 *   - ring: SVG circle that fills using stroke-dashoffset
 *   - text: Text element that displays the value
 *
 * Data attributes:
 *   data-gauge="type"       - Gauge type (bar, arc, polygon, ring, text)
 *   data-value-id="N"       - Placeholder ID for the current value
 *   data-max-id="N"         - Placeholder ID for the maximum value (optional)
 *   data-length="N"         - Path length for arc gauges (auto-calculated if omitted)
 *   data-perimeter="N"      - Perimeter for polygon gauges
 *   data-direction="dir"    - Fill direction: horizontal (default), vertical
 *   data-format="fmt"       - Text format: number (default), percent, fixed:N
 *   data-min="N"            - Minimum value (default: 0)
 *   data-invert="true"      - Invert the fill direction
 *
 * Example:
 *   <div class="bar-fill" data-gauge="bar" data-value-id="2" data-max-id="4"></div>
 *   <path data-gauge="arc" data-value-id="22" data-max-id="24" data-length="141"/>
 *
 * @version 2.0.0
 * @license MIT
 */

(function(global) {
    'use strict';

    // =========================================================================
    // Configuration
    // =========================================================================

    const CONFIG = {
        transitionDuration: 500,
        easing: 'ease-out',
        debug: false
    };

    function log(...args) {
        if (CONFIG.debug) {
            console.log('[rg-sens-gauges]', ...args);
        }
    }

    // =========================================================================
    // Utility Functions
    // =========================================================================

    function parseNumber(value, defaultValue = 0) {
        if (value === null || value === undefined || value === '') {
            return defaultValue;
        }
        const num = parseFloat(value);
        return isNaN(num) ? defaultValue : num;
    }

    function calcPercent(value, min, max) {
        const range = max - min;
        if (range <= 0) return 0;
        const percent = ((value - min) / range) * 100;
        return Math.min(100, Math.max(0, percent));
    }

    // =========================================================================
    // Gauge Update Functions (stateless - no persistent references)
    // =========================================================================

    function updateBarGauge(el, percent, direction) {
        if (direction === 'vertical') {
            el.style.height = percent + '%';
        } else {
            el.style.width = percent + '%';
        }
    }

    function updateStrokeGauge(el, percent, length) {
        const offset = length * (1 - percent / 100);
        el.setAttribute('stroke-dashoffset', String(offset));
    }

    function updateTextGauge(el, value, min, max, format) {
        let displayValue;
        switch (format) {
            case 'percent':
                const percent = calcPercent(value, min, max);
                displayValue = Math.round(percent) + '%';
                break;
            case 'integer':
                displayValue = Math.round(value).toString();
                break;
            default:
                if (format && format.startsWith('fixed:')) {
                    const decimals = parseInt(format.split(':')[1]) || 0;
                    displayValue = value.toFixed(decimals);
                } else {
                    displayValue = value.toString();
                }
        }
        el.textContent = displayValue;
    }

    function calculateLength(el, type) {
        switch (type) {
            case 'arc':
                if (typeof el.getTotalLength === 'function') {
                    try { return el.getTotalLength(); } catch (e) {}
                }
                return 141;
            case 'polygon':
                const points = el.getAttribute('points');
                if (!points) return 258;
                try {
                    const coords = points.trim().split(/\s+/).map(p => {
                        const [x, y] = p.split(',').map(parseFloat);
                        return { x, y };
                    });
                    let perimeter = 0;
                    for (let i = 0; i < coords.length; i++) {
                        const curr = coords[i];
                        const next = coords[(i + 1) % coords.length];
                        perimeter += Math.sqrt(
                            Math.pow(next.x - curr.x, 2) +
                            Math.pow(next.y - curr.y, 2)
                        );
                    }
                    return perimeter;
                } catch (e) {
                    return 258;
                }
            case 'ring':
                const r = parseFloat(el.getAttribute('r')) || 50;
                return 2 * Math.PI * r;
            default:
                return 100;
        }
    }

    // =========================================================================
    // Main Update Function (queries DOM fresh each time - no caching)
    // =========================================================================

    function updateValues(values) {
        if (!values || typeof values !== 'object') {
            return;
        }

        // Query all gauge elements fresh each time (no persistent references)
        const gaugeElements = document.querySelectorAll('[data-gauge]');

        gaugeElements.forEach(el => {
            const type = el.getAttribute('data-gauge');
            const valueId = el.getAttribute('data-value-id');
            const maxId = el.getAttribute('data-max-id');

            if (!valueId) return;

            // Get values
            const currentValue = parseNumber(values[valueId], 0);
            const maxValue = parseNumber(values[maxId], 100);
            const minValue = parseNumber(el.getAttribute('data-min'), 0);
            const invert = el.getAttribute('data-invert') === 'true';

            // Calculate percentage
            let percent = calcPercent(currentValue, minValue, maxValue);
            if (invert) {
                percent = 100 - percent;
            }

            // Update based on type
            switch (type) {
                case 'bar':
                    const direction = el.getAttribute('data-direction') || 'horizontal';
                    updateBarGauge(el, percent, direction);
                    break;

                case 'arc':
                case 'polygon':
                case 'ring':
                    let length = parseNumber(el.getAttribute('data-length') ||
                                            el.getAttribute('data-perimeter') ||
                                            el.getAttribute('data-circumference'), 0);
                    if (length === 0) {
                        length = calculateLength(el, type);
                        // Cache the calculated length on the element
                        el.setAttribute('data-length', String(length));
                    }
                    // Ensure stroke-dasharray is set
                    if (!el.getAttribute('stroke-dasharray')) {
                        el.setAttribute('stroke-dasharray', String(length));
                    }
                    updateStrokeGauge(el, percent, length);
                    break;

                case 'text':
                    const format = el.getAttribute('data-format') || 'number';
                    updateTextGauge(el, currentValue, minValue, maxValue, format);
                    break;
            }
        });

        log('Updated', gaugeElements.length, 'gauges');
    }

    // =========================================================================
    // Initialization
    // =========================================================================

    // Initialize stroke-dasharray on SVG elements (once on load)
    function initializeGauges() {
        const svgGauges = document.querySelectorAll('[data-gauge="arc"], [data-gauge="polygon"], [data-gauge="ring"]');
        svgGauges.forEach(el => {
            const type = el.getAttribute('data-gauge');
            let length = parseNumber(el.getAttribute('data-length') ||
                                    el.getAttribute('data-perimeter') ||
                                    el.getAttribute('data-circumference'), 0);
            if (length === 0) {
                length = calculateLength(el, type);
                el.setAttribute('data-length', String(length));
            }
            el.setAttribute('stroke-dasharray', String(length));
            el.setAttribute('stroke-dashoffset', String(length)); // Start empty
        });
        log('Initialized', svgGauges.length, 'SVG gauges');
    }

    // Expose updateValues globally
    global.updateValues = updateValues;

    // Expose configuration for advanced usage
    global.rgSensGauges = {
        updateValues: updateValues,
        setDebug: function(enabled) { CONFIG.debug = !!enabled; },
        reinitialize: initializeGauges
    };

    // Auto-initialize when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initializeGauges, { once: true });
    } else {
        initializeGauges();
    }

    log('rg-sens-gauges.js v2.0.0 loaded (memory-optimized)');

})(typeof window !== 'undefined' ? window : this);
