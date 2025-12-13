//! Custom color picker dialog with preset colors, RGB/HSV sliders, and saved colors

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, DrawingArea, Grid, Label, Orientation, Scale, SpinButton, Window,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::ui::background::Color;

const PRESET_COLORS: [[Color; 8]; 8] = [
    // Row 0: Very Light variations
    [
        Color { r: 0.8, g: 0.9, b: 1.0, a: 1.0 },   // Light Blue
        Color { r: 0.8, g: 1.0, b: 0.9, a: 1.0 },   // Light Green
        Color { r: 1.0, g: 1.0, b: 0.8, a: 1.0 },   // Light Yellow
        Color { r: 1.0, g: 0.9, b: 0.8, a: 1.0 },   // Light Orange
        Color { r: 1.0, g: 0.8, b: 0.8, a: 1.0 },   // Light Red
        Color { r: 0.95, g: 0.8, b: 1.0, a: 1.0 },  // Light Purple
        Color { r: 0.9, g: 0.8, b: 0.7, a: 1.0 },   // Light Brown
        Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },   // White
    ],
    // Row 1: Light variations
    [
        Color { r: 0.6, g: 0.8, b: 1.0, a: 1.0 },   // Blue tint
        Color { r: 0.6, g: 1.0, b: 0.8, a: 1.0 },   // Green tint
        Color { r: 1.0, g: 1.0, b: 0.6, a: 1.0 },   // Yellow tint
        Color { r: 1.0, g: 0.8, b: 0.6, a: 1.0 },   // Orange tint
        Color { r: 1.0, g: 0.6, b: 0.6, a: 1.0 },   // Red tint
        Color { r: 0.9, g: 0.6, b: 1.0, a: 1.0 },   // Purple tint
        Color { r: 0.8, g: 0.65, b: 0.5, a: 1.0 },  // Brown tint
        Color { r: 0.9, g: 0.9, b: 0.9, a: 1.0 },   // Light Gray
    ],
    // Row 2: Medium Light variations
    [
        Color { r: 0.4, g: 0.7, b: 1.0, a: 1.0 },   // Medium Light Blue
        Color { r: 0.4, g: 1.0, b: 0.7, a: 1.0 },   // Medium Light Green
        Color { r: 1.0, g: 0.95, b: 0.4, a: 1.0 },  // Medium Light Yellow
        Color { r: 1.0, g: 0.7, b: 0.4, a: 1.0 },   // Medium Light Orange
        Color { r: 1.0, g: 0.4, b: 0.4, a: 1.0 },   // Medium Light Red
        Color { r: 0.8, g: 0.4, b: 1.0, a: 1.0 },   // Medium Light Purple
        Color { r: 0.7, g: 0.55, b: 0.4, a: 1.0 },  // Medium Light Brown
        Color { r: 0.8, g: 0.8, b: 0.8, a: 1.0 },   // Gray
    ],
    // Row 3: Medium variations
    [
        Color { r: 0.2, g: 0.5, b: 1.0, a: 1.0 },   // Medium Blue
        Color { r: 0.2, g: 0.9, b: 0.5, a: 1.0 },   // Medium Green
        Color { r: 1.0, g: 0.9, b: 0.2, a: 1.0 },   // Medium Yellow
        Color { r: 1.0, g: 0.6, b: 0.2, a: 1.0 },   // Medium Orange
        Color { r: 1.0, g: 0.2, b: 0.2, a: 1.0 },   // Medium Red
        Color { r: 0.7, g: 0.2, b: 1.0, a: 1.0 },   // Medium Purple
        Color { r: 0.6, g: 0.45, b: 0.3, a: 1.0 },  // Medium Brown
        Color { r: 0.6, g: 0.6, b: 0.6, a: 1.0 },   // Medium Gray
    ],
    // Row 4: Standard/Saturated variations
    [
        Color { r: 0.0, g: 0.4, b: 1.0, a: 1.0 },   // Blue
        Color { r: 0.0, g: 0.8, b: 0.4, a: 1.0 },   // Green
        Color { r: 1.0, g: 0.8, b: 0.0, a: 1.0 },   // Yellow
        Color { r: 1.0, g: 0.5, b: 0.0, a: 1.0 },   // Orange
        Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 },   // Red
        Color { r: 0.6, g: 0.0, b: 1.0, a: 1.0 },   // Purple
        Color { r: 0.55, g: 0.35, b: 0.2, a: 1.0 }, // Brown
        Color { r: 0.5, g: 0.5, b: 0.5, a: 1.0 },   // Gray
    ],
    // Row 5: Dark variations
    [
        Color { r: 0.0, g: 0.3, b: 0.8, a: 1.0 },   // Dark Blue
        Color { r: 0.0, g: 0.6, b: 0.3, a: 1.0 },   // Dark Green
        Color { r: 0.8, g: 0.6, b: 0.0, a: 1.0 },   // Dark Yellow
        Color { r: 0.8, g: 0.4, b: 0.0, a: 1.0 },   // Dark Orange
        Color { r: 0.8, g: 0.0, b: 0.0, a: 1.0 },   // Dark Red
        Color { r: 0.5, g: 0.0, b: 0.8, a: 1.0 },   // Dark Purple
        Color { r: 0.4, g: 0.25, b: 0.1, a: 1.0 },  // Dark Brown
        Color { r: 0.4, g: 0.4, b: 0.4, a: 1.0 },   // Dark Gray
    ],
    // Row 6: Very Dark variations
    [
        Color { r: 0.0, g: 0.2, b: 0.6, a: 1.0 },   // Very Dark Blue
        Color { r: 0.0, g: 0.4, b: 0.2, a: 1.0 },   // Very Dark Green
        Color { r: 0.6, g: 0.4, b: 0.0, a: 1.0 },   // Very Dark Yellow
        Color { r: 0.6, g: 0.3, b: 0.0, a: 1.0 },   // Very Dark Orange
        Color { r: 0.6, g: 0.0, b: 0.0, a: 1.0 },   // Very Dark Red
        Color { r: 0.4, g: 0.0, b: 0.6, a: 1.0 },   // Very Dark Purple
        Color { r: 0.3, g: 0.2, b: 0.1, a: 1.0 },   // Very Dark Brown
        Color { r: 0.25, g: 0.25, b: 0.25, a: 1.0 },// Very Dark Gray
    ],
    // Row 7: Darkest variations
    [
        Color { r: 0.0, g: 0.1, b: 0.4, a: 1.0 },   // Darkest Blue
        Color { r: 0.0, g: 0.2, b: 0.1, a: 1.0 },   // Darkest Green
        Color { r: 0.4, g: 0.3, b: 0.0, a: 1.0 },   // Darkest Yellow
        Color { r: 0.4, g: 0.2, b: 0.0, a: 1.0 },   // Darkest Orange
        Color { r: 0.4, g: 0.0, b: 0.0, a: 1.0 },   // Darkest Red
        Color { r: 0.2, g: 0.0, b: 0.4, a: 1.0 },   // Darkest Purple
        Color { r: 0.2, g: 0.15, b: 0.1, a: 1.0 },  // Darkest Brown
        Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },   // Black
    ],
];

// Global saved colors storage (persistent across dialog instances)
// 16 slots in 2 rows of 8
thread_local! {
    static SAVED_COLORS: Rc<RefCell<Vec<Color>>> = Rc::new(RefCell::new(vec![Color::default(); 16]));
}

#[allow(dead_code)]
pub struct CustomColorPicker {
    dialog: Window,
    current_color: Rc<RefCell<Color>>,
    preview_area: DrawingArea,
    // RGB controls
    red_scale: Scale,
    green_scale: Scale,
    blue_scale: Scale,
    alpha_scale: Scale,
    red_spin: SpinButton,
    green_spin: SpinButton,
    blue_spin: SpinButton,
    alpha_spin: SpinButton,
    // HSV controls
    hue_scale: Scale,
    saturation_scale: Scale,
    value_scale: Scale,
    hue_spin: SpinButton,
    saturation_spin: SpinButton,
    value_spin: SpinButton,
    // Saved colors
    saved_colors_grid: Grid,
}

impl CustomColorPicker {
    pub fn new(parent: Option<&Window>, initial_color: Color) -> Self {
        let dialog = Window::builder()
            .title("Select Color")
            .modal(false)
            .default_width(850)
            .default_height(550)
            .resizable(true)
            .build();

        if let Some(parent) = parent {
            dialog.set_transient_for(Some(parent));
        }

        let current_color = Rc::new(RefCell::new(initial_color));

        let main_box = GtkBox::new(Orientation::Vertical, 12);
        main_box.set_margin_start(12);
        main_box.set_margin_end(12);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);

        // === Preview Area (at top, full width) ===
        let preview_area = DrawingArea::new();
        preview_area.set_size_request(-1, 60);  // Full width, 60px height
        preview_area.set_hexpand(true);
        preview_area.set_margin_bottom(12);

        let current_color_clone = current_color.clone();
        preview_area.set_draw_func(move |_, cr, width, height| {
            let color = *current_color_clone.borrow();

            // Draw checkerboard for transparency
            let checker_size = 16.0;
            for y in 0..(height / checker_size as i32 + 1) {
                for x in 0..(width / checker_size as i32 + 1) {
                    if (x + y) % 2 == 0 {
                        cr.set_source_rgb(0.8, 0.8, 0.8);
                    } else {
                        cr.set_source_rgb(0.6, 0.6, 0.6);
                    }
                    cr.rectangle(
                        x as f64 * checker_size,
                        y as f64 * checker_size,
                        checker_size,
                        checker_size,
                    );
                    cr.fill().ok();
                }
            }

            // Draw color with alpha
            cr.set_source_rgba(color.r, color.g, color.b, color.a);
            cr.rectangle(0.0, 0.0, width as f64, height as f64);
            cr.fill().ok();
        });

        main_box.append(&preview_area);

        // === Create RGB sliders first (we need them for color maps) ===
        let (_red_box, red_scale, red_spin) = Self::create_slider_with_spin("R", 0.0, 1.0, 0.01, initial_color.r);
        let (_green_box, green_scale, green_spin) = Self::create_slider_with_spin("G", 0.0, 1.0, 0.01, initial_color.g);
        let (_blue_box, blue_scale, blue_spin) = Self::create_slider_with_spin("B", 0.0, 1.0, 0.01, initial_color.b);
        let (_alpha_box, alpha_scale, alpha_spin) = Self::create_slider_with_spin("A", 0.0, 1.0, 0.01, initial_color.a);

        // === Create HSV sliders ===
        let (h, s, v) = rgb_to_hsv(initial_color.r, initial_color.g, initial_color.b);
        let (_hue_box, hue_scale, hue_spin) = Self::create_slider_with_spin("H", 0.0, 360.0, 1.0, h);
        let (_sat_box, saturation_scale, saturation_spin) = Self::create_slider_with_spin("S", 0.0, 1.0, 0.01, s);
        let (_val_box, value_scale, value_spin) = Self::create_slider_with_spin("V", 0.0, 1.0, 0.01, v);

        // === Main Content: Horizontal box with presets + sliders ===
        let content_box = GtkBox::new(Orientation::Horizontal, 12);

        // === LEFT SIDE: Preset Colors Grid (8x8) ===
        let preset_box = GtkBox::new(Orientation::Vertical, 6);
        let preset_label = Label::new(Some("Preset Colors"));
        preset_label.add_css_class("heading");
        preset_label.set_halign(gtk4::Align::Start);
        preset_box.append(&preset_label);

        let preset_grid = Grid::new();
        preset_grid.set_row_spacing(2);
        preset_grid.set_column_spacing(2);

        #[allow(clippy::needless_range_loop)]
        for row in 0..8 {
            for col in 0..8 {
                let color = PRESET_COLORS[row][col];
                let button = Self::create_color_button(
                    color,
                    current_color.clone(),
                    red_scale.clone(),
                    green_scale.clone(),
                    blue_scale.clone(),
                    alpha_scale.clone(),
                    hue_scale.clone(),
                    saturation_scale.clone(),
                    value_scale.clone(),
                    preview_area.clone(),
                );
                preset_grid.attach(&button, col as i32, row as i32, 1, 1);
            }
        }
        preset_box.append(&preset_grid);
        content_box.append(&preset_box);

        // === RIGHT SIDE: Horizontal sliders stacked vertically ===
        let sliders_box = GtkBox::new(Orientation::Vertical, 4);
        sliders_box.set_hexpand(true);

        // Alpha slider (at top)
        let alpha_label = Label::new(Some("Alpha"));
        alpha_label.set_halign(gtk4::Align::Start);
        alpha_label.add_css_class("heading");
        sliders_box.append(&alpha_label);
        sliders_box.append(&Self::create_horizontal_alpha_slider(&red_scale, &green_scale, &blue_scale, &alpha_scale));

        // Separator
        sliders_box.append(&gtk4::Separator::new(Orientation::Horizontal));

        // RGB section
        let rgb_label = Label::new(Some("RGB"));
        rgb_label.set_halign(gtk4::Align::Start);
        rgb_label.add_css_class("heading");
        sliders_box.append(&rgb_label);
        sliders_box.append(&Self::create_horizontal_rgb_slider("R", &red_scale, &green_scale, &blue_scale, 'r'));
        sliders_box.append(&Self::create_horizontal_rgb_slider("G", &red_scale, &green_scale, &blue_scale, 'g'));
        sliders_box.append(&Self::create_horizontal_rgb_slider("B", &red_scale, &green_scale, &blue_scale, 'b'));

        // Separator
        sliders_box.append(&gtk4::Separator::new(Orientation::Horizontal));

        // HSV section
        let hsv_label = Label::new(Some("HSV"));
        hsv_label.set_halign(gtk4::Align::Start);
        hsv_label.add_css_class("heading");
        sliders_box.append(&hsv_label);
        sliders_box.append(&Self::create_horizontal_hue_slider(&hue_scale));
        sliders_box.append(&Self::create_horizontal_saturation_slider(&hue_scale, &saturation_scale, &value_scale));
        sliders_box.append(&Self::create_horizontal_value_slider(&hue_scale, &saturation_scale, &value_scale));

        content_box.append(&sliders_box);
        main_box.append(&content_box);

        // === Saved Colors (16 slots in 2x8 grid) ===
        let saved_label = Label::new(Some("User Colors"));
        saved_label.add_css_class("heading");
        saved_label.set_halign(gtk4::Align::Start);
        saved_label.set_margin_top(6);
        main_box.append(&saved_label);

        let saved_colors_grid = Grid::new();
        saved_colors_grid.set_row_spacing(2);
        saved_colors_grid.set_column_spacing(2);
        saved_colors_grid.set_margin_bottom(6);

        // Initialize saved colors grid (2 rows of 8)
        SAVED_COLORS.with(|saved| {
            let colors = saved.borrow();
            for i in 0..16 {
                let row = i / 8;
                let col = i % 8;
                let color = colors[i];
                let button = Self::create_saved_color_button(
                    i,
                    color,
                    current_color.clone(),
                    red_scale.clone(),
                    green_scale.clone(),
                    blue_scale.clone(),
                    alpha_scale.clone(),
                    hue_scale.clone(),
                    saturation_scale.clone(),
                    value_scale.clone(),
                    preview_area.clone(),
                );
                saved_colors_grid.attach(&button, col as i32, row as i32, 1, 1);
            }
        });

        main_box.append(&saved_colors_grid);

        // Add "Add Color" button (stack-based: shifts right, drops last)
        let save_btn_box = GtkBox::new(Orientation::Horizontal, 6);
        let add_color_button = Button::with_label("Add Color");

        let current_color_for_save = current_color.clone();
        let saved_colors_grid_clone = saved_colors_grid.clone();
        add_color_button.connect_clicked(move |_| {
            let color = *current_color_for_save.borrow();

            SAVED_COLORS.with(|saved| {
                let mut colors = saved.borrow_mut();

                // Shift all colors to the right by one position
                for i in (1..16).rev() {
                    colors[i] = colors[i - 1];
                }

                // Add new color at position 0
                colors[0] = color;
            });

            // Save colors to disk
            if let Err(e) = Self::save_colors() {
                eprintln!("Failed to save user colors: {}", e);
            }

            // Update all button appearances
            for i in 0..16 {
                let row = i / 8;
                let col = i % 8;
                if let Some(child) = saved_colors_grid_clone.child_at(col as i32, row as i32) {
                    if let Ok(button) = child.downcast::<Button>() {
                        SAVED_COLORS.with(|saved| {
                            let colors = saved.borrow();
                            Self::update_color_button(&button, colors[i]);
                        });
                    }
                }
            }
        });

        save_btn_box.append(&add_color_button);
        main_box.append(&save_btn_box);

        // === Buttons ===
        let button_box = GtkBox::new(Orientation::Horizontal, 6);
        button_box.set_halign(gtk4::Align::End);
        button_box.set_margin_top(12);

        // Stock Color Picker button
        let stock_button = Button::with_label("Open Stock Color Picker...");
        button_box.append(&stock_button);

        let ok_button = Button::with_label("OK");
        ok_button.add_css_class("suggested-action");
        let cancel_button = Button::with_label("Cancel");

        button_box.append(&cancel_button);
        button_box.append(&ok_button);

        main_box.append(&button_box);

        dialog.set_child(Some(&main_box));

        let mut picker = Self {
            dialog,
            current_color,
            preview_area,
            red_scale,
            green_scale,
            blue_scale,
            alpha_scale,
            red_spin,
            green_spin,
            blue_spin,
            alpha_spin,
            hue_scale,
            saturation_scale,
            value_scale,
            hue_spin,
            saturation_spin,
            value_spin,
            saved_colors_grid,
        };

        // Wire up all the handlers
        picker.setup_rgb_handlers();
        picker.setup_hsv_handlers();
        picker.setup_button_handlers(ok_button, cancel_button, stock_button);

        picker
    }

    #[allow(clippy::too_many_arguments)]
    fn create_color_button(
        color: Color,
        current_color: Rc<RefCell<Color>>,
        red_scale: Scale,
        green_scale: Scale,
        blue_scale: Scale,
        alpha_scale: Scale,
        hue_scale: Scale,
        sat_scale: Scale,
        val_scale: Scale,
        preview: DrawingArea,
    ) -> Button {
        let button = Button::new();
        button.set_size_request(40, 40);

        Self::update_color_button(&button, color);

        button.connect_clicked(move |_| {
            *current_color.borrow_mut() = color;

            // Update RGB sliders
            red_scale.set_value(color.r);
            green_scale.set_value(color.g);
            blue_scale.set_value(color.b);
            alpha_scale.set_value(color.a);

            // Update HSV sliders
            let (h, s, v) = rgb_to_hsv(color.r, color.g, color.b);
            hue_scale.set_value(h);
            sat_scale.set_value(s);
            val_scale.set_value(v);

            // Redraw preview
            preview.queue_draw();
        });

        button
    }

    #[allow(clippy::too_many_arguments)]
    fn create_saved_color_button(
        slot: usize,
        color: Color,
        current_color: Rc<RefCell<Color>>,
        red_scale: Scale,
        green_scale: Scale,
        blue_scale: Scale,
        alpha_scale: Scale,
        hue_scale: Scale,
        sat_scale: Scale,
        val_scale: Scale,
        preview: DrawingArea,
    ) -> Button {
        let button = Button::new();
        button.set_size_request(40, 40);

        Self::update_color_button(&button, color);

        button.connect_clicked(move |_| {
            let color = SAVED_COLORS.with(|saved| {
                let colors = saved.borrow();
                colors[slot]
            });

            *current_color.borrow_mut() = color;

            // Update RGB sliders
            red_scale.set_value(color.r);
            green_scale.set_value(color.g);
            blue_scale.set_value(color.b);
            alpha_scale.set_value(color.a);

            // Update HSV sliders
            let (h, s, v) = rgb_to_hsv(color.r, color.g, color.b);
            hue_scale.set_value(h);
            sat_scale.set_value(s);
            val_scale.set_value(v);

            // Redraw preview
            preview.queue_draw();
        });

        button
    }

    fn update_color_button(button: &Button, color: Color) {
        let color_box = DrawingArea::new();
        color_box.set_size_request(40, 40);

        color_box.set_draw_func(move |_, cr, width, height| {
            // Draw checkerboard pattern for transparency
            let checker_size = 8.0;
            for y in 0..(height / checker_size as i32 + 1) {
                for x in 0..(width / checker_size as i32 + 1) {
                    if (x + y) % 2 == 0 {
                        cr.set_source_rgb(0.8, 0.8, 0.8);
                    } else {
                        cr.set_source_rgb(0.6, 0.6, 0.6);
                    }
                    cr.rectangle(
                        x as f64 * checker_size,
                        y as f64 * checker_size,
                        checker_size,
                        checker_size,
                    );
                    cr.fill().ok();
                }
            }

            // Draw color with alpha
            cr.set_source_rgba(color.r, color.g, color.b, color.a);
            cr.rectangle(0.0, 0.0, width as f64, height as f64);
            cr.fill().ok();
        });

        button.set_child(Some(&color_box));
    }

    fn create_slider_with_spin(
        label: &str,
        min: f64,
        max: f64,
        step: f64,
        value: f64,
    ) -> (GtkBox, Scale, SpinButton) {
        let hbox = GtkBox::new(Orientation::Horizontal, 6);
        let label_widget = Label::new(Some(label));
        label_widget.set_width_chars(3);
        hbox.append(&label_widget);

        let scale = Scale::with_range(Orientation::Horizontal, min, max, step);
        scale.set_value(value);
        scale.set_hexpand(true);
        scale.set_draw_value(false);
        hbox.append(&scale);

        let spin = SpinButton::with_range(min, max, step);
        spin.set_value(value);
        spin.set_width_chars(6);
        spin.set_digits(if step < 0.1 { 2 } else { 0 });
        hbox.append(&spin);

        // Sync scale and spin
        let spin_clone = spin.clone();
        scale.connect_value_changed(move |scale| {
            spin_clone.set_value(scale.value());
        });

        let scale_clone = scale.clone();
        spin.connect_value_changed(move |spin| {
            scale_clone.set_value(spin.value());
        });

        (hbox, scale, spin)
    }

    #[allow(dead_code)]
    fn create_vertical_rgb_slider(
        label: &str,
        red_scale: &Scale,
        green_scale: &Scale,
        blue_scale: &Scale,
        channel: char,
    ) -> GtkBox {
        let vbox = GtkBox::new(Orientation::Vertical, 6);
        vbox.set_size_request(50, -1);  // Constrain column width to 50px

        // Label at top
        let label_widget = Label::new(Some(label));
        label_widget.set_halign(gtk4::Align::Center);
        vbox.append(&label_widget);

        // Overlay container for color map with slider on top
        let overlay = gtk4::Overlay::new();

        // Color map (vertical gradient) - background
        let color_map = DrawingArea::new();
        color_map.set_size_request(20, 320);

        let red = red_scale.clone();
        let green = green_scale.clone();
        let blue = blue_scale.clone();

        color_map.set_draw_func(move |_, cr, width, height| {
            let r = red.value();
            let g = green.value();
            let b = blue.value();

            // Draw gradient from bottom (0.0) to top (1.0)
            let gradient = cairo::LinearGradient::new(0.0, height as f64, 0.0, 0.0);
            match channel {
                'r' => {
                    gradient.add_color_stop_rgb(0.0, 0.0, g, b);
                    gradient.add_color_stop_rgb(1.0, 1.0, g, b);
                }
                'g' => {
                    gradient.add_color_stop_rgb(0.0, r, 0.0, b);
                    gradient.add_color_stop_rgb(1.0, r, 1.0, b);
                }
                'b' => {
                    gradient.add_color_stop_rgb(0.0, r, g, 0.0);
                    gradient.add_color_stop_rgb(1.0, r, g, 1.0);
                }
                _ => {}
            }

            cr.set_source(&gradient).ok();
            cr.rectangle(0.0, 0.0, width as f64, height as f64);
            cr.fill().ok();
        });

        // Redraw when sliders change
        let color_map_clone = color_map.clone();
        red_scale.connect_value_changed(move |_| color_map_clone.queue_draw());
        let color_map_clone = color_map.clone();
        green_scale.connect_value_changed(move |_| color_map_clone.queue_draw());
        let color_map_clone = color_map.clone();
        blue_scale.connect_value_changed(move |_| color_map_clone.queue_draw());

        overlay.set_child(Some(&color_map));

        // Get the appropriate scale
        let scale = match channel {
            'r' => red_scale,
            'g' => green_scale,
            'b' => blue_scale,
            _ => red_scale,
        };

        // Vertical scale - overlay on top of color map
        let vertical_scale = Scale::with_range(Orientation::Vertical, 0.0, 1.0, 0.01);
        vertical_scale.set_value(scale.value());
        vertical_scale.set_inverted(true); // Top = 1.0, bottom = 0.0
        vertical_scale.set_vexpand(true);
        vertical_scale.set_draw_value(false);
        vertical_scale.set_opacity(0.8);  // Semi-transparent slider
        vertical_scale.set_size_request(20, 320);

        // Sync vertical scale with horizontal scale
        let scale_clone = scale.clone();
        vertical_scale.connect_value_changed(move |v_scale| {
            scale_clone.set_value(v_scale.value());
        });

        let vertical_scale_clone = vertical_scale.clone();
        scale.connect_value_changed(move |h_scale| {
            vertical_scale_clone.set_value(h_scale.value());
        });

        overlay.add_overlay(&vertical_scale);
        vbox.append(&overlay);

        // Spin button below (narrow to fit ~50px column width)
        let spin = SpinButton::with_range(0.0, 1.0, 0.01);
        spin.set_value(scale.value());
        spin.set_digits(2);
        spin.set_width_chars(4);
        spin.set_halign(gtk4::Align::Center);
        spin.set_size_request(50, -1);  // Max 50px width

        // Sync spin with scale
        let spin_clone = spin.clone();
        scale.connect_value_changed(move |s| {
            spin_clone.set_value(s.value());
        });

        let scale_clone = scale.clone();
        spin.connect_value_changed(move |sp| {
            scale_clone.set_value(sp.value());
        });

        vbox.append(&spin);

        vbox
    }

    #[allow(dead_code)]
    fn create_vertical_alpha_slider(
        label: &str,
        red_scale: &Scale,
        green_scale: &Scale,
        blue_scale: &Scale,
        alpha_scale: &Scale,
    ) -> GtkBox {
        let vbox = GtkBox::new(Orientation::Vertical, 6);
        vbox.set_size_request(50, -1);  // Constrain column width to 50px

        // Label at top
        let label_widget = Label::new(Some(label));
        label_widget.set_halign(gtk4::Align::Center);
        vbox.append(&label_widget);

        // Overlay container for color map with slider on top
        let overlay = gtk4::Overlay::new();

        // Color map with checkerboard (vertical gradient)
        let color_map = DrawingArea::new();
        color_map.set_size_request(20, 320);

        let red = red_scale.clone();
        let green = green_scale.clone();
        let blue = blue_scale.clone();

        color_map.set_draw_func(move |_, cr, width, height| {
            let r = red.value();
            let g = green.value();
            let b = blue.value();

            // Draw checkerboard
            let checker_size = 8.0;
            for y in 0..(height / checker_size as i32 + 1) {
                for x in 0..(width / checker_size as i32 + 1) {
                    if (x + y) % 2 == 0 {
                        cr.set_source_rgb(0.8, 0.8, 0.8);
                    } else {
                        cr.set_source_rgb(0.6, 0.6, 0.6);
                    }
                    let _ = cr.rectangle(
                        x as f64 * checker_size,
                        y as f64 * checker_size,
                        checker_size,
                        checker_size,
                    );
                    let _ = cr.fill();
                }
            }

            // Draw alpha gradient from bottom (0.0) to top (1.0)
            let gradient = cairo::LinearGradient::new(0.0, height as f64, 0.0, 0.0);
            gradient.add_color_stop_rgba(0.0, r, g, b, 0.0);
            gradient.add_color_stop_rgba(1.0, r, g, b, 1.0);
            let _ = cr.set_source(&gradient);
            let _ = cr.rectangle(0.0, 0.0, width as f64, height as f64);
            let _ = cr.fill();
        });

        // Redraw when RGB sliders change
        let color_map_clone = color_map.clone();
        red_scale.connect_value_changed(move |_| color_map_clone.queue_draw());
        let color_map_clone = color_map.clone();
        green_scale.connect_value_changed(move |_| color_map_clone.queue_draw());
        let color_map_clone = color_map.clone();
        blue_scale.connect_value_changed(move |_| color_map_clone.queue_draw());

        overlay.set_child(Some(&color_map));

        // Vertical scale - overlay on top of color map
        let vertical_scale = Scale::with_range(Orientation::Vertical, 0.0, 1.0, 0.01);
        vertical_scale.set_value(alpha_scale.value());
        vertical_scale.set_inverted(true);
        vertical_scale.set_vexpand(true);
        vertical_scale.set_draw_value(false);
        vertical_scale.set_opacity(0.8);
        vertical_scale.set_size_request(20, 320);

        // Sync vertical scale with horizontal scale
        let alpha_clone = alpha_scale.clone();
        vertical_scale.connect_value_changed(move |v_scale| {
            alpha_clone.set_value(v_scale.value());
        });

        let vertical_scale_clone = vertical_scale.clone();
        alpha_scale.connect_value_changed(move |h_scale| {
            vertical_scale_clone.set_value(h_scale.value());
        });

        overlay.add_overlay(&vertical_scale);
        vbox.append(&overlay);

        // Spin button below (narrow to fit ~50px column width)
        let spin = SpinButton::with_range(0.0, 1.0, 0.01);
        spin.set_value(alpha_scale.value());
        spin.set_digits(2);
        spin.set_width_chars(4);
        spin.set_halign(gtk4::Align::Center);
        spin.set_size_request(50, -1);  // Max 50px width

        // Sync spin with scale
        let spin_clone = spin.clone();
        alpha_scale.connect_value_changed(move |s| {
            spin_clone.set_value(s.value());
        });

        let alpha_clone = alpha_scale.clone();
        spin.connect_value_changed(move |sp| {
            alpha_clone.set_value(sp.value());
        });

        vbox.append(&spin);

        vbox
    }

    #[allow(dead_code)]
    fn create_vertical_hue_slider(hue_scale: &Scale) -> GtkBox {
        let vbox = GtkBox::new(Orientation::Vertical, 6);
        vbox.set_size_request(50, -1);  // Constrain column width to 50px

        // Label at top
        let label_widget = Label::new(Some("H"));
        label_widget.set_halign(gtk4::Align::Center);
        vbox.append(&label_widget);

        // Overlay container for color map with slider on top
        let overlay = gtk4::Overlay::new();

        // Hue color map (full spectrum vertically)
        let color_map = DrawingArea::new();
        color_map.set_size_request(20, 320);

        color_map.set_draw_func(move |_, cr, width, height| {
            // Draw full hue spectrum vertically
            let steps = height;
            for i in 0..steps {
                let hue = ((steps - i) as f64 / steps as f64) * 360.0; // Top = 360, bottom = 0
                let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
                cr.set_source_rgb(r, g, b);
                let _ = cr.rectangle(0.0, i as f64, width as f64, 1.0);
                let _ = cr.fill();
            }
        });

        overlay.set_child(Some(&color_map));

        // Vertical scale - overlay on top of color map
        let vertical_scale = Scale::with_range(Orientation::Vertical, 0.0, 360.0, 1.0);
        vertical_scale.set_value(hue_scale.value());
        vertical_scale.set_inverted(true);
        vertical_scale.set_vexpand(true);
        vertical_scale.set_draw_value(false);
        vertical_scale.set_opacity(0.8);
        vertical_scale.set_size_request(20, 320);

        // Sync vertical scale with horizontal scale
        let hue_clone = hue_scale.clone();
        vertical_scale.connect_value_changed(move |v_scale| {
            hue_clone.set_value(v_scale.value());
        });

        let vertical_scale_clone = vertical_scale.clone();
        hue_scale.connect_value_changed(move |h_scale| {
            vertical_scale_clone.set_value(h_scale.value());
        });

        overlay.add_overlay(&vertical_scale);
        vbox.append(&overlay);

        // Spin button below (narrow to fit ~50px column width)
        let spin = SpinButton::with_range(0.0, 360.0, 1.0);
        spin.set_value(hue_scale.value());
        spin.set_digits(0);
        spin.set_width_chars(4);
        spin.set_halign(gtk4::Align::Center);
        spin.set_size_request(50, -1);  // Max 50px width

        // Sync spin with scale
        let spin_clone = spin.clone();
        hue_scale.connect_value_changed(move |s| {
            spin_clone.set_value(s.value());
        });

        let hue_clone = hue_scale.clone();
        spin.connect_value_changed(move |sp| {
            hue_clone.set_value(sp.value());
        });

        vbox.append(&spin);

        vbox
    }

    #[allow(dead_code)]
    fn create_vertical_saturation_slider(
        hue_scale: &Scale,
        saturation_scale: &Scale,
        value_scale: &Scale,
    ) -> GtkBox {
        let vbox = GtkBox::new(Orientation::Vertical, 6);
        vbox.set_size_request(50, -1);  // Constrain column width to 50px

        // Label at top
        let label_widget = Label::new(Some("S"));
        label_widget.set_halign(gtk4::Align::Center);
        vbox.append(&label_widget);

        // Overlay container for color map with slider on top
        let overlay = gtk4::Overlay::new();

        // Saturation color map (white to full color vertically)
        let color_map = DrawingArea::new();
        color_map.set_size_request(20, 320);

        let hue = hue_scale.clone();
        let value = value_scale.clone();

        color_map.set_draw_func(move |_, cr, width, height| {
            let h = hue.value();
            let v = value.value();

            // Draw saturation gradient vertically (bottom = 0, top = 1)
            let gradient = cairo::LinearGradient::new(0.0, height as f64, 0.0, 0.0);
            let (r0, g0, b0) = hsv_to_rgb(h, 0.0, v);
            let (r1, g1, b1) = hsv_to_rgb(h, 1.0, v);
            gradient.add_color_stop_rgb(0.0, r0, g0, b0);
            gradient.add_color_stop_rgb(1.0, r1, g1, b1);
            let _ = cr.set_source(&gradient);
            let _ = cr.rectangle(0.0, 0.0, width as f64, height as f64);
            let _ = cr.fill();
        });

        // Redraw when hue or value changes
        let color_map_clone = color_map.clone();
        hue_scale.connect_value_changed(move |_| color_map_clone.queue_draw());
        let color_map_clone = color_map.clone();
        value_scale.connect_value_changed(move |_| color_map_clone.queue_draw());

        overlay.set_child(Some(&color_map));

        // Vertical scale - overlay on top of color map
        let vertical_scale = Scale::with_range(Orientation::Vertical, 0.0, 1.0, 0.01);
        vertical_scale.set_value(saturation_scale.value());
        vertical_scale.set_inverted(true);
        vertical_scale.set_vexpand(true);
        vertical_scale.set_draw_value(false);
        vertical_scale.set_opacity(0.8);
        vertical_scale.set_size_request(20, 320);

        // Sync vertical scale with horizontal scale
        let sat_clone = saturation_scale.clone();
        vertical_scale.connect_value_changed(move |v_scale| {
            sat_clone.set_value(v_scale.value());
        });

        let vertical_scale_clone = vertical_scale.clone();
        saturation_scale.connect_value_changed(move |h_scale| {
            vertical_scale_clone.set_value(h_scale.value());
        });

        overlay.add_overlay(&vertical_scale);
        vbox.append(&overlay);

        // Spin button below (narrow to fit ~50px column width)
        let spin = SpinButton::with_range(0.0, 1.0, 0.01);
        spin.set_value(saturation_scale.value());
        spin.set_digits(2);
        spin.set_width_chars(4);
        spin.set_halign(gtk4::Align::Center);
        spin.set_size_request(50, -1);  // Max 50px width

        // Sync spin with scale
        let spin_clone = spin.clone();
        saturation_scale.connect_value_changed(move |s| {
            spin_clone.set_value(s.value());
        });

        let sat_clone = saturation_scale.clone();
        spin.connect_value_changed(move |sp| {
            sat_clone.set_value(sp.value());
        });

        vbox.append(&spin);

        vbox
    }

    #[allow(dead_code)]
    fn create_vertical_value_slider(
        hue_scale: &Scale,
        saturation_scale: &Scale,
        value_scale: &Scale,
    ) -> GtkBox {
        let vbox = GtkBox::new(Orientation::Vertical, 6);
        vbox.set_size_request(50, -1);  // Constrain column width to 50px

        // Label at top
        let label_widget = Label::new(Some("V"));
        label_widget.set_halign(gtk4::Align::Center);
        vbox.append(&label_widget);

        // Overlay container for color map with slider on top
        let overlay = gtk4::Overlay::new();

        // Value color map (black to full color vertically)
        let color_map = DrawingArea::new();
        color_map.set_size_request(20, 320);

        let hue = hue_scale.clone();
        let saturation = saturation_scale.clone();

        color_map.set_draw_func(move |_, cr, width, height| {
            let h = hue.value();
            let s = saturation.value();

            // Draw value gradient vertically (bottom = 0, top = 1)
            let gradient = cairo::LinearGradient::new(0.0, height as f64, 0.0, 0.0);
            let (r0, g0, b0) = hsv_to_rgb(h, s, 0.0);
            let (r1, g1, b1) = hsv_to_rgb(h, s, 1.0);
            gradient.add_color_stop_rgb(0.0, r0, g0, b0);
            gradient.add_color_stop_rgb(1.0, r1, g1, b1);
            let _ = cr.set_source(&gradient);
            let _ = cr.rectangle(0.0, 0.0, width as f64, height as f64);
            let _ = cr.fill();
        });

        // Redraw when hue or saturation changes
        let color_map_clone = color_map.clone();
        hue_scale.connect_value_changed(move |_| color_map_clone.queue_draw());
        let color_map_clone = color_map.clone();
        saturation_scale.connect_value_changed(move |_| color_map_clone.queue_draw());

        overlay.set_child(Some(&color_map));

        // Vertical scale - overlay on top of color map
        let vertical_scale = Scale::with_range(Orientation::Vertical, 0.0, 1.0, 0.01);
        vertical_scale.set_value(value_scale.value());
        vertical_scale.set_inverted(true);
        vertical_scale.set_vexpand(true);
        vertical_scale.set_draw_value(false);
        vertical_scale.set_opacity(0.8);
        vertical_scale.set_size_request(20, 320);

        // Sync vertical scale with horizontal scale
        let val_clone = value_scale.clone();
        vertical_scale.connect_value_changed(move |v_scale| {
            val_clone.set_value(v_scale.value());
        });

        let vertical_scale_clone = vertical_scale.clone();
        value_scale.connect_value_changed(move |h_scale| {
            vertical_scale_clone.set_value(h_scale.value());
        });

        overlay.add_overlay(&vertical_scale);
        vbox.append(&overlay);

        // Spin button below (narrow to fit ~50px column width)
        let spin = SpinButton::with_range(0.0, 1.0, 0.01);
        spin.set_value(value_scale.value());
        spin.set_digits(2);
        spin.set_width_chars(4);
        spin.set_halign(gtk4::Align::Center);
        spin.set_size_request(50, -1);  // Max 50px width

        // Sync spin with scale
        let spin_clone = spin.clone();
        value_scale.connect_value_changed(move |s| {
            spin_clone.set_value(s.value());
        });

        let val_clone = value_scale.clone();
        spin.connect_value_changed(move |sp| {
            val_clone.set_value(sp.value());
        });

        vbox.append(&spin);

        vbox
    }

    /// Create a horizontal RGB slider with gradient background
    fn create_horizontal_rgb_slider(
        label: &str,
        red_scale: &Scale,
        green_scale: &Scale,
        blue_scale: &Scale,
        channel: char,
    ) -> GtkBox {
        let hbox = GtkBox::new(Orientation::Horizontal, 6);

        // Label
        let label_widget = Label::new(Some(label));
        label_widget.set_width_chars(2);
        hbox.append(&label_widget);

        // Overlay container for color map with slider on top
        let overlay = gtk4::Overlay::new();
        overlay.set_hexpand(true);

        // Color map (horizontal gradient) - background
        let color_map = DrawingArea::new();
        color_map.set_size_request(-1, 32);
        color_map.set_hexpand(true);

        let red = red_scale.clone();
        let green = green_scale.clone();
        let blue = blue_scale.clone();

        color_map.set_draw_func(move |_, cr, width, height| {
            let r = red.value();
            let g = green.value();
            let b = blue.value();

            // Draw horizontal gradient from left (0.0) to right (1.0)
            let gradient = cairo::LinearGradient::new(0.0, 0.0, width as f64, 0.0);
            match channel {
                'r' => {
                    gradient.add_color_stop_rgb(0.0, 0.0, g, b);
                    gradient.add_color_stop_rgb(1.0, 1.0, g, b);
                }
                'g' => {
                    gradient.add_color_stop_rgb(0.0, r, 0.0, b);
                    gradient.add_color_stop_rgb(1.0, r, 1.0, b);
                }
                'b' => {
                    gradient.add_color_stop_rgb(0.0, r, g, 0.0);
                    gradient.add_color_stop_rgb(1.0, r, g, 1.0);
                }
                _ => {}
            }

            let _ = cr.set_source(&gradient);
            let _ = cr.rectangle(0.0, 0.0, width as f64, height as f64);
            let _ = cr.fill();
        });

        // Redraw when sliders change
        let color_map_clone = color_map.clone();
        red_scale.connect_value_changed(move |_| color_map_clone.queue_draw());
        let color_map_clone = color_map.clone();
        green_scale.connect_value_changed(move |_| color_map_clone.queue_draw());
        let color_map_clone = color_map.clone();
        blue_scale.connect_value_changed(move |_| color_map_clone.queue_draw());

        overlay.set_child(Some(&color_map));

        // Get the appropriate scale
        let scale = match channel {
            'r' => red_scale,
            'g' => green_scale,
            'b' => blue_scale,
            _ => red_scale,
        };

        // Horizontal scale - overlay on top of color map
        let horizontal_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.01);
        horizontal_scale.set_value(scale.value());
        horizontal_scale.set_hexpand(true);
        horizontal_scale.set_draw_value(false);
        horizontal_scale.set_opacity(0.8);

        // Sync horizontal scale with main scale
        let scale_clone = scale.clone();
        horizontal_scale.connect_value_changed(move |h_scale| {
            scale_clone.set_value(h_scale.value());
        });

        let horizontal_scale_clone = horizontal_scale.clone();
        scale.connect_value_changed(move |s| {
            horizontal_scale_clone.set_value(s.value());
        });

        overlay.add_overlay(&horizontal_scale);
        hbox.append(&overlay);

        // Spin button
        let spin = SpinButton::with_range(0.0, 1.0, 0.01);
        spin.set_value(scale.value());
        spin.set_digits(2);
        spin.set_width_chars(5);

        // Sync spin with scale
        let spin_clone = spin.clone();
        scale.connect_value_changed(move |s| {
            spin_clone.set_value(s.value());
        });

        let scale_clone = scale.clone();
        spin.connect_value_changed(move |sp| {
            scale_clone.set_value(sp.value());
        });

        hbox.append(&spin);

        hbox
    }

    /// Create a horizontal alpha slider with gradient background
    fn create_horizontal_alpha_slider(
        red_scale: &Scale,
        green_scale: &Scale,
        blue_scale: &Scale,
        alpha_scale: &Scale,
    ) -> GtkBox {
        let hbox = GtkBox::new(Orientation::Horizontal, 6);

        // Label
        let label_widget = Label::new(Some("A"));
        label_widget.set_width_chars(2);
        hbox.append(&label_widget);

        // Overlay container for color map with slider on top
        let overlay = gtk4::Overlay::new();
        overlay.set_hexpand(true);

        // Color map with checkerboard (horizontal gradient)
        let color_map = DrawingArea::new();
        color_map.set_size_request(-1, 32);
        color_map.set_hexpand(true);

        let red = red_scale.clone();
        let green = green_scale.clone();
        let blue = blue_scale.clone();

        color_map.set_draw_func(move |_, cr, width, height| {
            let r = red.value();
            let g = green.value();
            let b = blue.value();

            // Draw checkerboard
            let checker_size = 8.0;
            for y in 0..(height / checker_size as i32 + 1) {
                for x in 0..(width / checker_size as i32 + 1) {
                    if (x + y) % 2 == 0 {
                        cr.set_source_rgb(0.8, 0.8, 0.8);
                    } else {
                        cr.set_source_rgb(0.6, 0.6, 0.6);
                    }
                    let _ = cr.rectangle(
                        x as f64 * checker_size,
                        y as f64 * checker_size,
                        checker_size,
                        checker_size,
                    );
                    let _ = cr.fill();
                }
            }

            // Draw alpha gradient horizontally
            let gradient = cairo::LinearGradient::new(0.0, 0.0, width as f64, 0.0);
            gradient.add_color_stop_rgba(0.0, r, g, b, 0.0);
            gradient.add_color_stop_rgba(1.0, r, g, b, 1.0);
            let _ = cr.set_source(&gradient);
            let _ = cr.rectangle(0.0, 0.0, width as f64, height as f64);
            let _ = cr.fill();
        });

        // Redraw when RGB sliders change
        let color_map_clone = color_map.clone();
        red_scale.connect_value_changed(move |_| color_map_clone.queue_draw());
        let color_map_clone = color_map.clone();
        green_scale.connect_value_changed(move |_| color_map_clone.queue_draw());
        let color_map_clone = color_map.clone();
        blue_scale.connect_value_changed(move |_| color_map_clone.queue_draw());

        overlay.set_child(Some(&color_map));

        // Horizontal scale - overlay on top of color map
        let horizontal_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.01);
        horizontal_scale.set_value(alpha_scale.value());
        horizontal_scale.set_hexpand(true);
        horizontal_scale.set_draw_value(false);
        horizontal_scale.set_opacity(0.8);

        // Sync horizontal scale with alpha scale
        let alpha_clone = alpha_scale.clone();
        horizontal_scale.connect_value_changed(move |h_scale| {
            alpha_clone.set_value(h_scale.value());
        });

        let horizontal_scale_clone = horizontal_scale.clone();
        alpha_scale.connect_value_changed(move |s| {
            horizontal_scale_clone.set_value(s.value());
        });

        overlay.add_overlay(&horizontal_scale);
        hbox.append(&overlay);

        // Spin button
        let spin = SpinButton::with_range(0.0, 1.0, 0.01);
        spin.set_value(alpha_scale.value());
        spin.set_digits(2);
        spin.set_width_chars(5);

        // Sync spin with scale
        let spin_clone = spin.clone();
        alpha_scale.connect_value_changed(move |s| {
            spin_clone.set_value(s.value());
        });

        let alpha_clone = alpha_scale.clone();
        spin.connect_value_changed(move |sp| {
            alpha_clone.set_value(sp.value());
        });

        hbox.append(&spin);

        hbox
    }

    /// Create a horizontal hue slider
    fn create_horizontal_hue_slider(hue_scale: &Scale) -> GtkBox {
        let hbox = GtkBox::new(Orientation::Horizontal, 6);

        // Label
        let label_widget = Label::new(Some("H"));
        label_widget.set_width_chars(2);
        hbox.append(&label_widget);

        // Overlay container for color map with slider on top
        let overlay = gtk4::Overlay::new();
        overlay.set_hexpand(true);

        // Hue color map (full spectrum horizontally)
        let color_map = DrawingArea::new();
        color_map.set_size_request(-1, 32);
        color_map.set_hexpand(true);

        color_map.set_draw_func(move |_, cr, width, height| {
            // Draw full hue spectrum horizontally
            let steps = width;
            for i in 0..steps {
                let hue = (i as f64 / steps as f64) * 360.0;
                let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
                cr.set_source_rgb(r, g, b);
                let _ = cr.rectangle(i as f64, 0.0, 1.0, height as f64);
                let _ = cr.fill();
            }
        });

        overlay.set_child(Some(&color_map));

        // Horizontal scale - overlay on top of color map
        let horizontal_scale = Scale::with_range(Orientation::Horizontal, 0.0, 360.0, 1.0);
        horizontal_scale.set_value(hue_scale.value());
        horizontal_scale.set_hexpand(true);
        horizontal_scale.set_draw_value(false);
        horizontal_scale.set_opacity(0.8);

        // Sync horizontal scale with hue scale
        let hue_clone = hue_scale.clone();
        horizontal_scale.connect_value_changed(move |h_scale| {
            hue_clone.set_value(h_scale.value());
        });

        let horizontal_scale_clone = horizontal_scale.clone();
        hue_scale.connect_value_changed(move |s| {
            horizontal_scale_clone.set_value(s.value());
        });

        overlay.add_overlay(&horizontal_scale);
        hbox.append(&overlay);

        // Spin button
        let spin = SpinButton::with_range(0.0, 360.0, 1.0);
        spin.set_value(hue_scale.value());
        spin.set_digits(0);
        spin.set_width_chars(5);

        // Sync spin with scale
        let spin_clone = spin.clone();
        hue_scale.connect_value_changed(move |s| {
            spin_clone.set_value(s.value());
        });

        let hue_clone = hue_scale.clone();
        spin.connect_value_changed(move |sp| {
            hue_clone.set_value(sp.value());
        });

        hbox.append(&spin);

        hbox
    }

    /// Create a horizontal saturation slider
    fn create_horizontal_saturation_slider(
        hue_scale: &Scale,
        saturation_scale: &Scale,
        value_scale: &Scale,
    ) -> GtkBox {
        let hbox = GtkBox::new(Orientation::Horizontal, 6);

        // Label
        let label_widget = Label::new(Some("S"));
        label_widget.set_width_chars(2);
        hbox.append(&label_widget);

        // Overlay container for color map with slider on top
        let overlay = gtk4::Overlay::new();
        overlay.set_hexpand(true);

        // Saturation color map
        let color_map = DrawingArea::new();
        color_map.set_size_request(-1, 32);
        color_map.set_hexpand(true);

        let hue = hue_scale.clone();
        let value = value_scale.clone();

        color_map.set_draw_func(move |_, cr, width, height| {
            let h = hue.value();
            let v = value.value();

            // Draw saturation gradient horizontally
            let gradient = cairo::LinearGradient::new(0.0, 0.0, width as f64, 0.0);
            let (r0, g0, b0) = hsv_to_rgb(h, 0.0, v);
            let (r1, g1, b1) = hsv_to_rgb(h, 1.0, v);
            gradient.add_color_stop_rgb(0.0, r0, g0, b0);
            gradient.add_color_stop_rgb(1.0, r1, g1, b1);
            let _ = cr.set_source(&gradient);
            let _ = cr.rectangle(0.0, 0.0, width as f64, height as f64);
            let _ = cr.fill();
        });

        // Redraw when hue or value changes
        let color_map_clone = color_map.clone();
        hue_scale.connect_value_changed(move |_| color_map_clone.queue_draw());
        let color_map_clone = color_map.clone();
        value_scale.connect_value_changed(move |_| color_map_clone.queue_draw());

        overlay.set_child(Some(&color_map));

        // Horizontal scale - overlay on top of color map
        let horizontal_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.01);
        horizontal_scale.set_value(saturation_scale.value());
        horizontal_scale.set_hexpand(true);
        horizontal_scale.set_draw_value(false);
        horizontal_scale.set_opacity(0.8);

        // Sync horizontal scale with saturation scale
        let sat_clone = saturation_scale.clone();
        horizontal_scale.connect_value_changed(move |h_scale| {
            sat_clone.set_value(h_scale.value());
        });

        let horizontal_scale_clone = horizontal_scale.clone();
        saturation_scale.connect_value_changed(move |s| {
            horizontal_scale_clone.set_value(s.value());
        });

        overlay.add_overlay(&horizontal_scale);
        hbox.append(&overlay);

        // Spin button
        let spin = SpinButton::with_range(0.0, 1.0, 0.01);
        spin.set_value(saturation_scale.value());
        spin.set_digits(2);
        spin.set_width_chars(5);

        // Sync spin with scale
        let spin_clone = spin.clone();
        saturation_scale.connect_value_changed(move |s| {
            spin_clone.set_value(s.value());
        });

        let sat_clone = saturation_scale.clone();
        spin.connect_value_changed(move |sp| {
            sat_clone.set_value(sp.value());
        });

        hbox.append(&spin);

        hbox
    }

    /// Create a horizontal value slider
    fn create_horizontal_value_slider(
        hue_scale: &Scale,
        saturation_scale: &Scale,
        value_scale: &Scale,
    ) -> GtkBox {
        let hbox = GtkBox::new(Orientation::Horizontal, 6);

        // Label
        let label_widget = Label::new(Some("V"));
        label_widget.set_width_chars(2);
        hbox.append(&label_widget);

        // Overlay container for color map with slider on top
        let overlay = gtk4::Overlay::new();
        overlay.set_hexpand(true);

        // Value color map
        let color_map = DrawingArea::new();
        color_map.set_size_request(-1, 32);
        color_map.set_hexpand(true);

        let hue = hue_scale.clone();
        let saturation = saturation_scale.clone();

        color_map.set_draw_func(move |_, cr, width, height| {
            let h = hue.value();
            let s = saturation.value();

            // Draw value gradient horizontally
            let gradient = cairo::LinearGradient::new(0.0, 0.0, width as f64, 0.0);
            let (r0, g0, b0) = hsv_to_rgb(h, s, 0.0);
            let (r1, g1, b1) = hsv_to_rgb(h, s, 1.0);
            gradient.add_color_stop_rgb(0.0, r0, g0, b0);
            gradient.add_color_stop_rgb(1.0, r1, g1, b1);
            let _ = cr.set_source(&gradient);
            let _ = cr.rectangle(0.0, 0.0, width as f64, height as f64);
            let _ = cr.fill();
        });

        // Redraw when hue or saturation changes
        let color_map_clone = color_map.clone();
        hue_scale.connect_value_changed(move |_| color_map_clone.queue_draw());
        let color_map_clone = color_map.clone();
        saturation_scale.connect_value_changed(move |_| color_map_clone.queue_draw());

        overlay.set_child(Some(&color_map));

        // Horizontal scale - overlay on top of color map
        let horizontal_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.01);
        horizontal_scale.set_value(value_scale.value());
        horizontal_scale.set_hexpand(true);
        horizontal_scale.set_draw_value(false);
        horizontal_scale.set_opacity(0.8);

        // Sync horizontal scale with value scale
        let val_clone = value_scale.clone();
        horizontal_scale.connect_value_changed(move |h_scale| {
            val_clone.set_value(h_scale.value());
        });

        let horizontal_scale_clone = horizontal_scale.clone();
        value_scale.connect_value_changed(move |s| {
            horizontal_scale_clone.set_value(s.value());
        });

        overlay.add_overlay(&horizontal_scale);
        hbox.append(&overlay);

        // Spin button
        let spin = SpinButton::with_range(0.0, 1.0, 0.01);
        spin.set_value(value_scale.value());
        spin.set_digits(2);
        spin.set_width_chars(5);

        // Sync spin with scale
        let spin_clone = spin.clone();
        value_scale.connect_value_changed(move |s| {
            spin_clone.set_value(s.value());
        });

        let val_clone = value_scale.clone();
        spin.connect_value_changed(move |sp| {
            val_clone.set_value(sp.value());
        });

        hbox.append(&spin);

        hbox
    }

    fn setup_rgb_handlers(&mut self) {
        let updating = Rc::new(RefCell::new(false));

        // RGB -> Color + HSV
        let current_color = self.current_color.clone();
        let preview = self.preview_area.clone();
        let updating_clone = updating.clone();
        let hue_scale = self.hue_scale.clone();
        let sat_scale = self.saturation_scale.clone();
        let val_scale = self.value_scale.clone();
        let red_scale = self.red_scale.clone();
        let green_scale = self.green_scale.clone();
        let blue_scale = self.blue_scale.clone();
        let alpha_scale = self.alpha_scale.clone();

        let update_from_rgb = move || {
            if *updating_clone.borrow() {
                return;
            }
            *updating_clone.borrow_mut() = true;

            let r = red_scale.value();
            let g = green_scale.value();
            let b = blue_scale.value();
            let a = alpha_scale.value();

            *current_color.borrow_mut() = Color { r, g, b, a };

            // Update HSV
            let (h, s, v) = rgb_to_hsv(r, g, b);
            hue_scale.set_value(h);
            sat_scale.set_value(s);
            val_scale.set_value(v);

            preview.queue_draw();

            *updating_clone.borrow_mut() = false;
        };

        let update_clone1 = update_from_rgb.clone();
        let update_clone2 = update_from_rgb.clone();
        let update_clone3 = update_from_rgb.clone();
        let update_clone4 = update_from_rgb;

        self.red_scale.connect_value_changed(move |_| update_clone1());
        self.green_scale.connect_value_changed(move |_| update_clone2());
        self.blue_scale.connect_value_changed(move |_| update_clone3());
        self.alpha_scale.connect_value_changed(move |_| update_clone4());
    }

    fn setup_hsv_handlers(&mut self) {
        let updating = Rc::new(RefCell::new(false));

        // HSV -> RGB + Color
        let current_color = self.current_color.clone();
        let preview = self.preview_area.clone();
        let updating_clone = updating.clone();
        let red_scale = self.red_scale.clone();
        let green_scale = self.green_scale.clone();
        let blue_scale = self.blue_scale.clone();
        let alpha_scale = self.alpha_scale.clone();
        let hue_scale = self.hue_scale.clone();
        let sat_scale = self.saturation_scale.clone();
        let val_scale = self.value_scale.clone();

        let update_from_hsv = move || {
            if *updating_clone.borrow() {
                return;
            }
            *updating_clone.borrow_mut() = true;

            let h = hue_scale.value();
            let s = sat_scale.value();
            let v = val_scale.value();
            let a = alpha_scale.value();

            let (r, g, b) = hsv_to_rgb(h, s, v);

            *current_color.borrow_mut() = Color { r, g, b, a };

            // Update RGB
            red_scale.set_value(r);
            green_scale.set_value(g);
            blue_scale.set_value(b);

            preview.queue_draw();

            *updating_clone.borrow_mut() = false;
        };

        let update_clone1 = update_from_hsv.clone();
        let update_clone2 = update_from_hsv.clone();
        let update_clone3 = update_from_hsv;

        self.hue_scale.connect_value_changed(move |_| update_clone1());
        self.saturation_scale.connect_value_changed(move |_| update_clone2());
        self.value_scale.connect_value_changed(move |_| update_clone3());
    }

    fn setup_button_handlers(
        &self,
        ok_button: Button,
        cancel_button: Button,
        stock_button: Button,
    ) {
        let dialog_clone = self.dialog.clone();
        cancel_button.connect_clicked(move |_| {
            dialog_clone.close();
        });

        let dialog_clone = self.dialog.clone();
        ok_button.connect_clicked(move |_| {
            dialog_clone.close();
        });

        // Stock color picker button
        let dialog = self.dialog.clone();
        let current_color = self.current_color.clone();
        let red_scale = self.red_scale.clone();
        let green_scale = self.green_scale.clone();
        let blue_scale = self.blue_scale.clone();
        let alpha_scale = self.alpha_scale.clone();

        stock_button.connect_clicked(move |_| {
            let color = *current_color.borrow();
            let dialog_parent = dialog.clone().upcast::<Window>();
            let red_scale_clone = red_scale.clone();
            let green_scale_clone = green_scale.clone();
            let blue_scale_clone = blue_scale.clone();
            let alpha_scale_clone = alpha_scale.clone();

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Some(new_color) = crate::ui::ColorPickerDialog::pick_color_stock(Some(&dialog_parent), color).await {
                    red_scale_clone.set_value(new_color.r);
                    green_scale_clone.set_value(new_color.g);
                    blue_scale_clone.set_value(new_color.b);
                    alpha_scale_clone.set_value(new_color.a);
                }
            });
        });
    }

    pub async fn pick_color(parent: Option<&Window>, initial_color: Color) -> Option<Color> {
        use std::future::Future;
        use std::pin::Pin;
        use std::task::{Context, Poll, Waker};

        let picker = Self::new(parent, initial_color);
        let result = Rc::new(RefCell::new(None));
        let waker = Rc::new(RefCell::new(None::<Waker>));

        let result_clone = result.clone();
        let waker_clone = waker.clone();
        let current_color = picker.current_color.clone();

        // Handle close to capture result and wake the future
        picker.dialog.connect_close_request(move |_| {
            *result_clone.borrow_mut() = Some(*current_color.borrow());
            if let Some(waker) = waker_clone.borrow_mut().take() {
                waker.wake();
            }
            gtk4::glib::Propagation::Proceed
        });

        picker.dialog.present();

        // Create a future that waits for the dialog to close
        struct DialogFuture {
            result: Rc<RefCell<Option<Color>>>,
            waker: Rc<RefCell<Option<Waker>>>,
        }

        impl Future for DialogFuture {
            type Output = Option<Color>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if let Some(color) = *self.result.borrow() {
                    Poll::Ready(Some(color))
                } else {
                    *self.waker.borrow_mut() = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        }

        DialogFuture { result, waker }.await
    }

    /// Save user colors to disk
    pub fn save_colors() -> anyhow::Result<()> {
        let colors = SAVED_COLORS.with(|saved| saved.borrow().clone());

        let config_path = Self::user_colors_path()?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&colors)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// Load user colors from disk
    pub fn load_colors() -> anyhow::Result<()> {
        let config_path = Self::user_colors_path()?;

        if !config_path.exists() {
            return Ok(()); // No saved colors yet
        }

        let content = std::fs::read_to_string(config_path)?;
        let colors: Vec<Color> = serde_json::from_str(&content)?;

        // Ensure we have exactly 16 colors
        let colors = if colors.len() < 16 {
            let mut padded = colors;
            padded.resize(16, Color::default());
            padded
        } else if colors.len() > 16 {
            colors[..16].to_vec()
        } else {
            colors
        };

        SAVED_COLORS.with(|saved| {
            *saved.borrow_mut() = colors;
        });

        Ok(())
    }

    /// Get the path to the user colors config file
    fn user_colors_path() -> anyhow::Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("com", "github.hilgardt_collab", "rg-sens")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

        Ok(dirs.config_dir().join("user_colors.json"))
    }
}

// HSV/RGB conversion utilities
fn rgb_to_hsv(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let v = max;

    let s = if max == 0.0 { 0.0 } else { delta / max };

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let h = if h < 0.0 { h + 360.0 } else { h };

    (h, s, v)
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (f64, f64, f64) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (r + m, g + m, b + m)
}
