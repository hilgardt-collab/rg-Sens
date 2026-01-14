/**
 * rg-sens-gauges.js
 *
 * A lightweight JavaScript library for CSS template panel visualizations.
 * Handles bars, arcs, polygons, and ring gauges with smooth animations.
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
 *   data-value="{{N}}"      - Placeholder for the current value
 *   data-max="{{N}}"        - Placeholder for the maximum value (optional)
 *   data-value-id="N"       - Alternative: specify placeholder ID directly
 *   data-max-id="N"         - Alternative: specify max placeholder ID directly
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
 * @version 1.0.0
 * @license MIT
 */

(function(global) {
    'use strict';

    // =========================================================================
    // Configuration
    // =========================================================================

    const CONFIG = {
        // Default transition duration in milliseconds
        transitionDuration: 500,

        // Default easing function (CSS transition-timing-function)
        easing: 'ease-out',

        // Attribute prefix for data attributes
        attrPrefix: 'data-',

        // Debug mode - set to true for console logging
        debug: false
    };

    // =========================================================================
    // Utility Functions
    // =========================================================================

    /**
     * Log debug messages if debug mode is enabled
     */
    function log(...args) {
        if (CONFIG.debug) {
            console.log('[rg-sens-gauges]', ...args);
        }
    }

    /**
     * Parse a numeric value from various input types
     */
    function parseNumber(value, defaultValue = 0) {
        if (value === null || value === undefined || value === '') {
            return defaultValue;
        }
        const num = parseFloat(value);
        return isNaN(num) ? defaultValue : num;
    }

    /**
     * Extract placeholder ID from a string like "{{5}}" or just "5"
     */
    function extractPlaceholderId(str) {
        if (!str) return null;
        // Match {{N}} pattern
        const match = str.match(/\{\{(\d+)\}\}/);
        if (match) {
            return match[1];
        }
        // Check if it's already just a number
        if (/^\d+$/.test(str.trim())) {
            return str.trim();
        }
        return null;
    }

    /**
     * Calculate percentage, clamped between 0 and 100
     */
    function calcPercent(value, min, max) {
        const range = max - min;
        if (range <= 0) return 0;
        const percent = ((value - min) / range) * 100;
        return Math.min(100, Math.max(0, percent));
    }

    /**
     * Calculate SVG path length for an arc
     * For a semicircle arc: length ≈ π * radius
     */
    function calculateArcLength(pathElement) {
        if (pathElement && typeof pathElement.getTotalLength === 'function') {
            try {
                return pathElement.getTotalLength();
            } catch (e) {
                log('Could not calculate path length:', e);
            }
        }
        return 141; // Default for typical arc gauges
    }

    /**
     * Calculate polygon perimeter
     */
    function calculatePolygonPerimeter(polygonElement) {
        if (!polygonElement) return 258; // Default for hexagon

        try {
            const points = polygonElement.getAttribute('points');
            if (!points) return 258;

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
            log('Could not calculate polygon perimeter:', e);
            return 258;
        }
    }

    /**
     * Calculate circle circumference
     */
    function calculateCircleCircumference(circleElement) {
        if (!circleElement) return 314; // Default for r=50
        const r = parseFloat(circleElement.getAttribute('r')) || 50;
        return 2 * Math.PI * r;
    }

    // =========================================================================
    // Gauge Class
    // =========================================================================

    class Gauge {
        constructor(element) {
            this.element = element;
            this.type = element.getAttribute('data-gauge');
            this.valueId = null;
            this.maxId = null;
            this.currentValue = 0;
            this.maxValue = 100;
            this.minValue = 0;
            this.length = 0;
            this.direction = 'horizontal';
            this.format = 'number';
            this.invert = false;

            this._parseAttributes();
            this._initialize();
        }

        /**
         * Parse data attributes from the element
         */
        _parseAttributes() {
            const el = this.element;

            // Get value ID - either from data-value-id or extract from data-value
            this.valueId = el.getAttribute('data-value-id') ||
                           extractPlaceholderId(el.getAttribute('data-value'));

            // Get max ID - either from data-max-id or extract from data-max
            this.maxId = el.getAttribute('data-max-id') ||
                         extractPlaceholderId(el.getAttribute('data-max'));

            // Get min value
            this.minValue = parseNumber(el.getAttribute('data-min'), 0);

            // Get direction for bars
            this.direction = el.getAttribute('data-direction') || 'horizontal';

            // Get format for text gauges
            this.format = el.getAttribute('data-format') || 'number';

            // Get invert flag
            this.invert = el.getAttribute('data-invert') === 'true';

            // Get length/perimeter based on type
            switch (this.type) {
                case 'arc':
                    this.length = parseNumber(
                        el.getAttribute('data-length'),
                        calculateArcLength(el)
                    );
                    break;
                case 'polygon':
                    this.length = parseNumber(
                        el.getAttribute('data-perimeter'),
                        calculatePolygonPerimeter(el)
                    );
                    break;
                case 'ring':
                    this.length = parseNumber(
                        el.getAttribute('data-circumference'),
                        calculateCircleCircumference(el)
                    );
                    break;
            }

            log('Parsed gauge:', {
                type: this.type,
                valueId: this.valueId,
                maxId: this.maxId,
                length: this.length,
                direction: this.direction
            });
        }

        /**
         * Initialize the gauge with default styling
         */
        _initialize() {
            const el = this.element;

            switch (this.type) {
                case 'bar':
                    // Set initial state to empty (0%)
                    if (this.direction === 'vertical') {
                        el.style.height = '0%';
                    } else {
                        el.style.width = '0%';
                    }
                    // Ensure transition is set
                    if (!el.style.transition) {
                        const prop = this.direction === 'vertical' ? 'height' : 'width';
                        el.style.transition = `${prop} ${CONFIG.transitionDuration}ms ${CONFIG.easing}`;
                    }
                    break;

                case 'arc':
                case 'polygon':
                case 'ring':
                    // Set initial stroke-dasharray and dashoffset as strings for SVG compatibility
                    el.setAttribute('stroke-dasharray', String(this.length));
                    el.setAttribute('stroke-dashoffset', String(this.length)); // Start empty
                    // Note: CSS transitions should be defined in the stylesheet for SVG elements
                    break;
            }
        }

        /**
         * Update the gauge with new values
         */
        update(values) {
            // Try both string and numeric keys for compatibility
            const valueIdStr = String(this.valueId);
            const maxIdStr = String(this.maxId);

            // Get current value from values object
            const rawValue = values[valueIdStr] !== undefined ? values[valueIdStr] : values[this.valueId];
            if (this.valueId && rawValue !== undefined) {
                this.currentValue = parseNumber(rawValue, this.currentValue);
            }

            // Get max value from values object (if specified)
            const rawMax = values[maxIdStr] !== undefined ? values[maxIdStr] : values[this.maxId];
            if (this.maxId && rawMax !== undefined) {
                this.maxValue = parseNumber(rawMax, this.maxValue);
            }

            // Calculate percentage
            let percent = calcPercent(this.currentValue, this.minValue, this.maxValue);

            // Invert if requested
            if (this.invert) {
                percent = 100 - percent;
            }

            // Apply based on gauge type
            switch (this.type) {
                case 'bar':
                    this._updateBar(percent);
                    break;
                case 'arc':
                case 'polygon':
                case 'ring':
                    this._updateStroke(percent);
                    break;
                case 'text':
                    this._updateText();
                    break;
            }
        }

        /**
         * Update a bar gauge
         */
        _updateBar(percent) {
            if (this.direction === 'vertical') {
                this.element.style.height = percent + '%';
            } else {
                this.element.style.width = percent + '%';
            }
        }

        /**
         * Update a stroke-based gauge (arc, polygon, ring)
         */
        _updateStroke(percent) {
            const offset = this.length * (1 - percent / 100);
            // Use setAttribute for better SVG/WebKit compatibility
            this.element.setAttribute('stroke-dashoffset', String(offset));
        }

        /**
         * Update a text gauge
         */
        _updateText() {
            let displayValue;

            switch (this.format) {
                case 'percent':
                    const percent = calcPercent(this.currentValue, this.minValue, this.maxValue);
                    displayValue = Math.round(percent) + '%';
                    break;
                case 'integer':
                    displayValue = Math.round(this.currentValue).toString();
                    break;
                default:
                    // Check for fixed:N format
                    if (this.format.startsWith('fixed:')) {
                        const decimals = parseInt(this.format.split(':')[1]) || 0;
                        displayValue = this.currentValue.toFixed(decimals);
                    } else {
                        displayValue = this.currentValue.toString();
                    }
            }

            this.element.textContent = displayValue;
        }
    }

    // =========================================================================
    // GaugeManager Class
    // =========================================================================

    class GaugeManager {
        constructor() {
            this.gauges = [];
            this.valueElements = new Map(); // Map of placeholder ID -> elements
            this.initialized = false;
        }

        /**
         * Initialize the manager - scan DOM for gauges
         */
        init() {
            if (this.initialized) {
                log('Already initialized, re-scanning...');
            }

            this.gauges = [];
            this.valueElements.clear();

            // Find all gauge elements
            const gaugeElements = document.querySelectorAll('[data-gauge]');
            gaugeElements.forEach(el => {
                const gauge = new Gauge(el);
                if (gauge.valueId) {
                    this.gauges.push(gauge);
                }
            });

            // Find all elements with placeholder text content that need updating
            // These are elements that contain {{N}} patterns
            this._scanForPlaceholderElements();

            this.initialized = true;
            log('Initialized with', this.gauges.length, 'gauges and',
                this.valueElements.size, 'value elements');

            return this;
        }

        /**
         * Scan for elements containing {{N}} placeholders for text updates
         */
        _scanForPlaceholderElements() {
            const walker = document.createTreeWalker(
                document.body,
                NodeFilter.SHOW_TEXT,
                null,
                false
            );

            let node;
            while (node = walker.nextNode()) {
                const text = node.textContent;
                const matches = text.match(/\{\{(\d+)\}\}/g);
                if (matches) {
                    matches.forEach(match => {
                        const id = match.replace(/[{}]/g, '');
                        if (!this.valueElements.has(id)) {
                            this.valueElements.set(id, []);
                        }
                        // Store reference to parent element and the original template
                        const parent = node.parentElement;
                        if (parent && !parent.hasAttribute('data-gauge')) {
                            this.valueElements.get(id).push({
                                element: parent,
                                template: parent.innerHTML
                            });
                        }
                    });
                }
            }
        }

        /**
         * Update all gauges with new values
         */
        updateValues(values) {
            if (!this.initialized) {
                this.init();
            }

            if (!values || typeof values !== 'object') {
                log('updateValues called with invalid values:', values);
                return;
            }

            log('Updating values:', Object.keys(values).length, 'values for', this.gauges.length, 'gauges');

            // Update all gauges
            this.gauges.forEach(gauge => {
                gauge.update(values);
            });

            // Update text elements with placeholder values
            // Note: This is handled by rg-Sens before calling updateValues,
            // but we support it here for standalone testing
            this.valueElements.forEach((elements, id) => {
                if (values[id] !== undefined) {
                    elements.forEach(({ element, template }) => {
                        // Only update if the template still has placeholders
                        // (rg-Sens replaces them, but for testing we might want this)
                        if (template.includes('{{')) {
                            let newHtml = template;
                            Object.keys(values).forEach(key => {
                                newHtml = newHtml.replace(
                                    new RegExp(`\\{\\{${key}\\}\\}`, 'g'),
                                    values[key]
                                );
                            });
                            element.innerHTML = newHtml;
                        }
                    });
                }
            });
        }

        /**
         * Get a gauge by its value ID
         */
        getGauge(valueId) {
            return this.gauges.find(g => g.valueId === valueId);
        }

        /**
         * Manually add a gauge element
         */
        addGauge(element) {
            const gauge = new Gauge(element);
            if (gauge.valueId) {
                this.gauges.push(gauge);
            }
            return gauge;
        }

        /**
         * Enable or disable debug mode
         */
        setDebug(enabled) {
            CONFIG.debug = !!enabled;
        }

        /**
         * Set global transition duration
         */
        setTransitionDuration(ms) {
            CONFIG.transitionDuration = ms;
        }
    }

    // =========================================================================
    // Initialization
    // =========================================================================

    // Create global manager instance
    const manager = new GaugeManager();

    // Expose the updateValues function globally
    global.updateValues = function(values) {
        manager.updateValues(values);
    };

    // Expose the manager for advanced usage
    global.rgSensGauges = manager;

    // Auto-initialize when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => {
            manager.init();
            // Trigger initial update with empty values to set initial states
            manager.updateValues({});
        });
    } else {
        // DOM already loaded
        manager.init();
        manager.updateValues({});
    }

    log('rg-sens-gauges.js loaded');

})(typeof window !== 'undefined' ? window : this);
