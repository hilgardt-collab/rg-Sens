//! Cyberpunk HUD configuration widget
//!
//! Provides a tabbed interface for configuring all aspects of the Cyberpunk display.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Entry, Label, Notebook, Orientation,
    Scale, SpinButton, StringList, ScrolledWindow,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::shared_font_dialog::shared_font_dialog;
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::cyberpunk_display::{
    render_cyberpunk_frame, CornerStyle, HeaderStyle, DividerStyle,
};
use crate::ui::graph_config_widget::GraphConfigWidget;
use crate::ui::bar_config_widget::BarConfigWidget;
use crate::ui::core_bars_config_widget::CoreBarsConfigWidget;
use crate::ui::lcars_display::{ContentDisplayType, SplitOrientation};
use crate::displayers::CyberpunkDisplayConfig;
use crate::core::FieldMetadata;

/// Holds references to Frame tab widgets for updating when config changes
struct FrameWidgets {
    border_width_spin: SpinButton,
    border_color_widget: Rc<ColorButtonWidget>,
    glow_spin: SpinButton,
    corner_style_dropdown: DropDown,
    corner_size_spin: SpinButton,
    bg_color_widget: Rc<ColorButtonWidget>,
    padding_spin: SpinButton,
}

/// Holds references to Effects tab widgets
struct EffectsWidgets {
    show_grid_check: CheckButton,
    grid_color_widget: Rc<ColorButtonWidget>,
    grid_spacing_spin: SpinButton,
    show_scanlines_check: CheckButton,
    scanline_opacity_spin: SpinButton,
    item_frame_check: CheckButton,
    item_frame_color_widget: Rc<ColorButtonWidget>,
    item_glow_check: CheckButton,
}

/// Holds references to Header tab widgets
struct HeaderWidgets {
    show_header_check: CheckButton,
    header_text_entry: Entry,
    header_style_dropdown: DropDown,
    header_color_widget: Rc<ColorButtonWidget>,
    header_font_btn: Button,
    header_font_size_spin: SpinButton,
}

/// Holds references to Layout tab widgets
struct LayoutWidgets {
    orientation_dropdown: DropDown,
    divider_style_dropdown: DropDown,
    divider_color_widget: Rc<ColorButtonWidget>,
    divider_width_spin: SpinButton,
    group_weights_box: GtkBox,
}

/// Holds references to Animation tab widgets
struct AnimationWidgets {
    enable_check: CheckButton,
    speed_scale: Scale,
}

/// Cyberpunk configuration widget
pub struct CyberpunkConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<CyberpunkDisplayConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: DrawingArea,
    /// Notebook for per-slot content configuration
    content_notebook: Rc<RefCell<Notebook>>,
    /// Source summaries for labeling tabs (slot_name, summary, group_num, item_idx)
    source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
    /// Available fields from the source for text overlay configuration
    available_fields: Rc<RefCell<Vec<FieldMetadata>>>,
    /// Frame tab widgets
    frame_widgets: Rc<RefCell<Option<FrameWidgets>>>,
    /// Effects tab widgets
    effects_widgets: Rc<RefCell<Option<EffectsWidgets>>>,
    /// Header tab widgets
    header_widgets: Rc<RefCell<Option<HeaderWidgets>>>,
    /// Layout tab widgets
    layout_widgets: Rc<RefCell<Option<LayoutWidgets>>>,
    /// Animation tab widgets
    animation_widgets: Rc<RefCell<Option<AnimationWidgets>>>,
}

impl CyberpunkConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        log::info!("=== CyberpunkConfigWidget::new() called with {} fields ===", available_fields.len());
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(CyberpunkDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>> = Rc::new(RefCell::new(Vec::new()));
        let available_fields: Rc<RefCell<Vec<FieldMetadata>>> = Rc::new(RefCell::new(available_fields));
        let frame_widgets: Rc<RefCell<Option<FrameWidgets>>> = Rc::new(RefCell::new(None));
        let effects_widgets: Rc<RefCell<Option<EffectsWidgets>>> = Rc::new(RefCell::new(None));
        let header_widgets: Rc<RefCell<Option<HeaderWidgets>>> = Rc::new(RefCell::new(None));
        let layout_widgets: Rc<RefCell<Option<LayoutWidgets>>> = Rc::new(RefCell::new(None));
        let animation_widgets: Rc<RefCell<Option<AnimationWidgets>>> = Rc::new(RefCell::new(None));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(200);
        preview.set_vexpand(false);

        let config_clone = config.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            // Skip if dimensions too small
            if width < 10 || height < 10 {
                return;
            }

            // Dark background for preview
            cr.set_source_rgb(0.05, 0.05, 0.1);
            cr.paint().ok();

            let cfg = config_clone.borrow();
            let _ = render_cyberpunk_frame(cr, &cfg.frame, width as f64, height as f64);
        });

        // Create notebook for tabbed interface
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // Tab 1: Frame
        let frame_page = Self::create_frame_page(&config, &on_change, &preview, &frame_widgets);
        notebook.append_page(&frame_page, Some(&Label::new(Some("Frame"))));

        // Tab 2: Effects
        let effects_page = Self::create_effects_page(&config, &on_change, &preview, &effects_widgets);
        notebook.append_page(&effects_page, Some(&Label::new(Some("Effects"))));

        // Tab 3: Header
        let header_page = Self::create_header_page(&config, &on_change, &preview, &header_widgets);
        notebook.append_page(&header_page, Some(&Label::new(Some("Header"))));

        // Tab 4: Layout
        let layout_page = Self::create_layout_page(&config, &on_change, &preview, &layout_widgets);
        notebook.append_page(&layout_page, Some(&Label::new(Some("Layout"))));

        // Tab 5: Content - with dynamic per-slot notebook
        let content_notebook = Rc::new(RefCell::new(Notebook::new()));
        let content_page = Self::create_content_page(&config, &on_change, &preview, &content_notebook, &source_summaries, &available_fields);
        notebook.append_page(&content_page, Some(&Label::new(Some("Content"))));

        // Tab 6: Animation
        let animation_page = Self::create_animation_page(&config, &on_change, &animation_widgets);
        notebook.append_page(&animation_page, Some(&Label::new(Some("Animation"))));

        container.append(&preview);
        container.append(&notebook);

        Self {
            container,
            config,
            on_change,
            preview,
            content_notebook,
            source_summaries,
            available_fields,
            frame_widgets,
            effects_widgets,
            header_widgets,
            layout_widgets,
            animation_widgets,
        }
    }

    fn set_page_margins(page: &GtkBox) {
        page.set_margin_start(12);
        page.set_margin_end(12);
        page.set_margin_top(12);
        page.set_margin_bottom(12);
    }

    fn queue_redraw(
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) {
        preview.queue_draw();
        if let Some(cb) = on_change.borrow().as_ref() {
            cb();
        }
    }

    fn create_frame_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        frame_widgets_out: &Rc<RefCell<Option<FrameWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Border width
        let border_box = GtkBox::new(Orientation::Horizontal, 6);
        border_box.append(&Label::new(Some("Border Width:")));
        let border_width_spin = SpinButton::with_range(0.5, 10.0, 0.5);
        border_width_spin.set_value(config.borrow().frame.border_width);
        border_width_spin.set_hexpand(true);
        border_box.append(&border_width_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        border_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.border_width = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&border_box);

        // Border color
        let color_box = GtkBox::new(Orientation::Horizontal, 6);
        color_box.append(&Label::new(Some("Border Color:")));
        let border_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.border_color));
        color_box.append(border_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        border_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.border_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&color_box);

        // Glow intensity
        let glow_box = GtkBox::new(Orientation::Horizontal, 6);
        glow_box.append(&Label::new(Some("Glow Intensity:")));
        let glow_spin = SpinButton::with_range(0.0, 1.0, 0.1);
        glow_spin.set_digits(2);
        glow_spin.set_value(config.borrow().frame.glow_intensity);
        glow_spin.set_hexpand(true);
        glow_box.append(&glow_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        glow_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.glow_intensity = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&glow_box);

        // Corner style
        let corner_style_box = GtkBox::new(Orientation::Horizontal, 6);
        corner_style_box.append(&Label::new(Some("Corner Style:")));
        let corner_list = StringList::new(&["Chamfer (45°)", "Bracket [ ]", "Angular"]);
        let corner_style_dropdown = DropDown::new(Some(corner_list), None::<gtk4::Expression>);
        let corner_idx = match config.borrow().frame.corner_style {
            CornerStyle::Chamfer => 0,
            CornerStyle::Bracket => 1,
            CornerStyle::Angular => 2,
        };
        corner_style_dropdown.set_selected(corner_idx);
        corner_style_dropdown.set_hexpand(true);
        corner_style_box.append(&corner_style_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        corner_style_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.corner_style = match dropdown.selected() {
                0 => CornerStyle::Chamfer,
                1 => CornerStyle::Bracket,
                _ => CornerStyle::Angular,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&corner_style_box);

        // Corner size
        let corner_size_box = GtkBox::new(Orientation::Horizontal, 6);
        corner_size_box.append(&Label::new(Some("Corner Size:")));
        let corner_size_spin = SpinButton::with_range(4.0, 50.0, 2.0);
        corner_size_spin.set_value(config.borrow().frame.corner_size);
        corner_size_spin.set_hexpand(true);
        corner_size_box.append(&corner_size_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        corner_size_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.corner_size = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&corner_size_box);

        // Background color
        let bg_box = GtkBox::new(Orientation::Horizontal, 6);
        bg_box.append(&Label::new(Some("Background:")));
        let bg_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.background_color));
        bg_box.append(bg_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bg_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.background_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&bg_box);

        // Content padding
        let padding_box = GtkBox::new(Orientation::Horizontal, 6);
        padding_box.append(&Label::new(Some("Content Padding:")));
        let padding_spin = SpinButton::with_range(0.0, 50.0, 2.0);
        padding_spin.set_value(config.borrow().frame.content_padding);
        padding_spin.set_hexpand(true);
        padding_box.append(&padding_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        padding_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.content_padding = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&padding_box);

        // Store widget refs
        *frame_widgets_out.borrow_mut() = Some(FrameWidgets {
            border_width_spin,
            border_color_widget,
            glow_spin,
            corner_style_dropdown,
            corner_size_spin,
            bg_color_widget,
            padding_spin,
        });

        page
    }

    fn create_effects_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        effects_widgets_out: &Rc<RefCell<Option<EffectsWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Grid section
        let grid_label = Label::new(Some("Grid Pattern"));
        grid_label.set_halign(gtk4::Align::Start);
        grid_label.add_css_class("heading");
        page.append(&grid_label);

        // Show grid
        let show_grid_check = CheckButton::with_label("Show Grid");
        show_grid_check.set_active(config.borrow().frame.show_grid);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_grid_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_grid = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&show_grid_check);

        // Grid color
        let grid_color_box = GtkBox::new(Orientation::Horizontal, 6);
        grid_color_box.append(&Label::new(Some("Grid Color:")));
        let grid_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.grid_color));
        grid_color_box.append(grid_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        grid_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.grid_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&grid_color_box);

        // Grid spacing
        let grid_spacing_box = GtkBox::new(Orientation::Horizontal, 6);
        grid_spacing_box.append(&Label::new(Some("Grid Spacing:")));
        let grid_spacing_spin = SpinButton::with_range(5.0, 100.0, 5.0);
        grid_spacing_spin.set_value(config.borrow().frame.grid_spacing);
        grid_spacing_spin.set_hexpand(true);
        grid_spacing_box.append(&grid_spacing_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        grid_spacing_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.grid_spacing = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&grid_spacing_box);

        // Scanlines section
        let scanline_label = Label::new(Some("Scanlines (CRT Effect)"));
        scanline_label.set_halign(gtk4::Align::Start);
        scanline_label.add_css_class("heading");
        scanline_label.set_margin_top(12);
        page.append(&scanline_label);

        // Show scanlines
        let show_scanlines_check = CheckButton::with_label("Show Scanlines");
        show_scanlines_check.set_active(config.borrow().frame.show_scanlines);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_scanlines_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_scanlines = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&show_scanlines_check);

        // Scanline opacity
        let scanline_opacity_box = GtkBox::new(Orientation::Horizontal, 6);
        scanline_opacity_box.append(&Label::new(Some("Opacity:")));
        let scanline_opacity_spin = SpinButton::with_range(0.0, 0.5, 0.02);
        scanline_opacity_spin.set_digits(2);
        scanline_opacity_spin.set_value(config.borrow().frame.scanline_opacity);
        scanline_opacity_spin.set_hexpand(true);
        scanline_opacity_box.append(&scanline_opacity_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        scanline_opacity_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.scanline_opacity = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&scanline_opacity_box);

        // Item frames section
        let item_frame_label = Label::new(Some("Content Item Frames"));
        item_frame_label.set_halign(gtk4::Align::Start);
        item_frame_label.add_css_class("heading");
        item_frame_label.set_margin_top(12);
        page.append(&item_frame_label);

        // Enable item frames
        let item_frame_check = CheckButton::with_label("Show Item Frames");
        item_frame_check.set_active(config.borrow().frame.item_frame_enabled);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        item_frame_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.item_frame_enabled = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&item_frame_check);

        // Item frame color
        let item_frame_color_box = GtkBox::new(Orientation::Horizontal, 6);
        item_frame_color_box.append(&Label::new(Some("Frame Color:")));
        let item_frame_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.item_frame_color));
        item_frame_color_box.append(item_frame_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        item_frame_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.item_frame_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&item_frame_color_box);

        // Item frame glow
        let item_glow_check = CheckButton::with_label("Item Frame Glow");
        item_glow_check.set_active(config.borrow().frame.item_glow_enabled);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        item_glow_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.item_glow_enabled = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&item_glow_check);

        // Store widget refs
        *effects_widgets_out.borrow_mut() = Some(EffectsWidgets {
            show_grid_check,
            grid_color_widget,
            grid_spacing_spin,
            show_scanlines_check,
            scanline_opacity_spin,
            item_frame_check,
            item_frame_color_widget,
            item_glow_check,
        });

        page
    }

    fn create_header_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        header_widgets_out: &Rc<RefCell<Option<HeaderWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Show header
        let show_header_check = CheckButton::with_label("Show Header");
        show_header_check.set_active(config.borrow().frame.show_header);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        show_header_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.show_header = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&show_header_check);

        // Header text
        let text_box = GtkBox::new(Orientation::Horizontal, 6);
        text_box.append(&Label::new(Some("Header Text:")));
        let header_text_entry = Entry::new();
        header_text_entry.set_text(&config.borrow().frame.header_text);
        header_text_entry.set_hexpand(true);
        text_box.append(&header_text_entry);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_text_entry.connect_changed(move |entry| {
            config_clone.borrow_mut().frame.header_text = entry.text().to_string();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&text_box);

        // Header style
        let style_box = GtkBox::new(Orientation::Horizontal, 6);
        style_box.append(&Label::new(Some("Style:")));
        let style_list = StringList::new(&["Brackets", "Underline", "Box", "None"]);
        let header_style_dropdown = DropDown::new(Some(style_list), None::<gtk4::Expression>);
        let style_idx = match config.borrow().frame.header_style {
            HeaderStyle::Brackets => 0,
            HeaderStyle::Underline => 1,
            HeaderStyle::Box => 2,
            HeaderStyle::None => 3,
        };
        header_style_dropdown.set_selected(style_idx);
        header_style_dropdown.set_hexpand(true);
        style_box.append(&header_style_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_style_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.header_style = match dropdown.selected() {
                0 => HeaderStyle::Brackets,
                1 => HeaderStyle::Underline,
                2 => HeaderStyle::Box,
                _ => HeaderStyle::None,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&style_box);

        // Header color
        let color_box = GtkBox::new(Orientation::Horizontal, 6);
        color_box.append(&Label::new(Some("Text Color:")));
        let header_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.header_color));
        color_box.append(header_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.header_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&color_box);

        // Header font
        let font_box = GtkBox::new(Orientation::Horizontal, 6);
        font_box.append(&Label::new(Some("Font:")));
        let header_font_btn = Button::with_label(&config.borrow().frame.header_font);
        header_font_btn.set_hexpand(true);
        font_box.append(&header_font_btn);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let font_btn_clone = header_font_btn.clone();
        header_font_btn.connect_clicked(move |btn| {
            let root = btn.root();
            let window = root.as_ref().and_then(|r| r.downcast_ref::<gtk4::Window>());
            let current_font = config_clone.borrow().frame.header_font.clone();
            let config_for_cb = config_clone.clone();
            let on_change_for_cb = on_change_clone.clone();
            let preview_for_cb = preview_clone.clone();
            let font_btn_for_cb = font_btn_clone.clone();

            // Create font description from current font family
            let font_desc = gtk4::pango::FontDescription::from_string(&current_font);

            shared_font_dialog().choose_font(
                window,
                Some(&font_desc),
                gtk4::gio::Cancellable::NONE,
                move |result| {
                    if let Ok(font_desc) = result {
                        let family = font_desc.family().map(|s| s.to_string()).unwrap_or_else(|| "Sans".to_string());
                        config_for_cb.borrow_mut().frame.header_font = family.clone();
                        font_btn_for_cb.set_label(&family);
                        Self::queue_redraw(&preview_for_cb, &on_change_for_cb);
                    }
                },
            );
        });
        page.append(&font_box);

        // Header font size
        let size_box = GtkBox::new(Orientation::Horizontal, 6);
        size_box.append(&Label::new(Some("Font Size:")));
        let header_font_size_spin = SpinButton::with_range(8.0, 48.0, 1.0);
        header_font_size_spin.set_value(config.borrow().frame.header_font_size);
        header_font_size_spin.set_hexpand(true);
        size_box.append(&header_font_size_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        header_font_size_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.header_font_size = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&size_box);

        // Store widget refs
        *header_widgets_out.borrow_mut() = Some(HeaderWidgets {
            show_header_check,
            header_text_entry,
            header_style_dropdown,
            header_color_widget,
            header_font_btn,
            header_font_size_spin,
        });

        page
    }

    fn create_layout_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        layout_widgets_out: &Rc<RefCell<Option<LayoutWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Layout section
        let layout_label = Label::new(Some("Layout"));
        layout_label.set_halign(gtk4::Align::Start);
        layout_label.add_css_class("heading");
        page.append(&layout_label);

        // Orientation
        let orient_box = GtkBox::new(Orientation::Horizontal, 6);
        orient_box.append(&Label::new(Some("Split Direction:")));
        let orient_list = StringList::new(&["Vertical", "Horizontal"]);
        let orientation_dropdown = DropDown::new(Some(orient_list), None::<gtk4::Expression>);
        let orient_idx = match config.borrow().frame.split_orientation {
            SplitOrientation::Vertical => 0,
            SplitOrientation::Horizontal => 1,
        };
        orientation_dropdown.set_selected(orient_idx);
        orientation_dropdown.set_hexpand(true);
        orient_box.append(&orientation_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        orientation_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.split_orientation = match dropdown.selected() {
                0 => SplitOrientation::Vertical,
                _ => SplitOrientation::Horizontal,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&orient_box);

        // Group weights section
        let weights_label = Label::new(Some("Group Size Weights"));
        weights_label.set_halign(gtk4::Align::Start);
        weights_label.add_css_class("heading");
        weights_label.set_margin_top(12);
        page.append(&weights_label);

        let weights_info = Label::new(Some("Groups are configured in the Data Sources tab."));
        weights_info.set_halign(gtk4::Align::Start);
        weights_info.add_css_class("dim-label");
        page.append(&weights_info);

        let group_weights_box = GtkBox::new(Orientation::Vertical, 4);
        page.append(&group_weights_box);

        // Dividers section
        let dividers_label = Label::new(Some("Dividers"));
        dividers_label.set_halign(gtk4::Align::Start);
        dividers_label.add_css_class("heading");
        dividers_label.set_margin_top(12);
        page.append(&dividers_label);

        // Divider style
        let div_style_box = GtkBox::new(Orientation::Horizontal, 6);
        div_style_box.append(&Label::new(Some("Style:")));
        let div_style_list = StringList::new(&["Line", "Dashed", "Glow", "Dots", "None"]);
        let divider_style_dropdown = DropDown::new(Some(div_style_list), None::<gtk4::Expression>);
        let div_style_idx = match config.borrow().frame.divider_style {
            DividerStyle::Line => 0,
            DividerStyle::Dashed => 1,
            DividerStyle::Glow => 2,
            DividerStyle::Dots => 3,
            DividerStyle::None => 4,
        };
        divider_style_dropdown.set_selected(div_style_idx);
        divider_style_dropdown.set_hexpand(true);
        div_style_box.append(&divider_style_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        divider_style_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.divider_style = match dropdown.selected() {
                0 => DividerStyle::Line,
                1 => DividerStyle::Dashed,
                2 => DividerStyle::Glow,
                3 => DividerStyle::Dots,
                _ => DividerStyle::None,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&div_style_box);

        // Divider color
        let div_color_box = GtkBox::new(Orientation::Horizontal, 6);
        div_color_box.append(&Label::new(Some("Color:")));
        let divider_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.divider_color));
        div_color_box.append(divider_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        divider_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.divider_color = color;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&div_color_box);

        // Divider width
        let div_width_box = GtkBox::new(Orientation::Horizontal, 6);
        div_width_box.append(&Label::new(Some("Width:")));
        let divider_width_spin = SpinButton::with_range(0.5, 5.0, 0.5);
        divider_width_spin.set_value(config.borrow().frame.divider_width);
        divider_width_spin.set_hexpand(true);
        div_width_box.append(&divider_width_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        divider_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.divider_width = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&div_width_box);

        // Initial build of group weight spinners
        Self::rebuild_group_spinners(config, on_change, preview, &group_weights_box);

        // Store widget refs
        *layout_widgets_out.borrow_mut() = Some(LayoutWidgets {
            orientation_dropdown,
            divider_style_dropdown,
            divider_color_widget,
            divider_width_spin,
            group_weights_box,
        });

        page
    }

    fn rebuild_group_spinners(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        weights_box: &GtkBox,
    ) {
        // Clear existing spinners
        while let Some(child) = weights_box.first_child() {
            weights_box.remove(&child);
        }

        let cfg = config.borrow();
        let count = cfg.frame.group_count;

        if count == 0 {
            let placeholder = Label::new(Some("No groups configured.\nAdd sources in the Data Sources tab."));
            placeholder.set_halign(gtk4::Align::Start);
            placeholder.add_css_class("dim-label");
            weights_box.append(&placeholder);
            return;
        }

        // Build weight spinners
        for i in 0..count {
            let row = GtkBox::new(Orientation::Horizontal, 6);
            row.append(&Label::new(Some(&format!("Group {}:", i + 1))));

            let weight_spin = SpinButton::with_range(0.1, 10.0, 0.1);
            weight_spin.set_digits(1);
            weight_spin.set_value(cfg.frame.group_size_weights.get(i).copied().unwrap_or(1.0));
            weight_spin.set_hexpand(true);
            row.append(&weight_spin);

            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            let preview_clone = preview.clone();
            weight_spin.connect_value_changed(move |spin| {
                let mut cfg = config_clone.borrow_mut();
                if i < cfg.frame.group_size_weights.len() {
                    cfg.frame.group_size_weights[i] = spin.value();
                }
                Self::queue_redraw(&preview_clone, &on_change_clone);
            });

            weights_box.append(&row);
        }
    }

    fn create_content_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        let info_label = Label::new(Some("Configure content items for each slot.\nSlots are named: group1_1, group1_2, group2_1, etc."));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        page.append(&info_label);

        let scrolled = ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_hexpand(true);
        scrolled.set_min_content_height(300);

        let notebook = content_notebook.borrow();
        scrolled.set_child(Some(&*notebook));
        page.append(&scrolled);

        // Initial build of content tabs
        drop(notebook);
        Self::rebuild_content_tabs(config, on_change, preview, content_notebook, source_summaries, available_fields);

        page
    }

    fn rebuild_content_tabs(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
    ) {
        let notebook = content_notebook.borrow();

        // Clear existing tabs
        while notebook.n_pages() > 0 {
            notebook.remove_page(Some(0));
        }

        let summaries = source_summaries.borrow();
        if summaries.is_empty() {
            // No sources configured yet - show placeholder
            let placeholder = Label::new(Some("No content sources configured.\nAdd sources in the combo source settings."));
            placeholder.set_halign(gtk4::Align::Center);
            placeholder.set_valign(gtk4::Align::Center);
            notebook.append_page(&placeholder, Some(&Label::new(Some("No Sources"))));
            return;
        }

        // Create a tab for each slot
        for (slot_name, summary, _group_num, _item_idx) in summaries.iter() {
            let tab_content = Self::create_slot_config_tab(
                config,
                on_change,
                preview,
                slot_name,
                available_fields,
            );

            let tab_label = if summary.is_empty() {
                slot_name.clone()
            } else {
                format!("{}\n{}", slot_name, summary)
            };
            notebook.append_page(&tab_content, Some(&Label::new(Some(&tab_label))));
        }
    }

    fn create_slot_config_tab(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        slot_name: &str,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        page.set_margin_start(8);
        page.set_margin_end(8);
        page.set_margin_top(8);
        page.set_margin_bottom(8);

        let slot_name_owned = slot_name.to_string();

        // Display type dropdown
        let type_box = GtkBox::new(Orientation::Horizontal, 6);
        type_box.append(&Label::new(Some("Display As:")));
        let type_list = StringList::new(&["Bar", "Text", "Graph", "Core Bars", "Static"]);
        let type_dropdown = DropDown::new(Some(type_list), None::<gtk4::Expression>);

        // Get current display type
        let current_type = {
            let cfg = config.borrow();
            cfg.frame.content_items.get(&slot_name_owned)
                .map(|item| match item.display_as {
                    ContentDisplayType::Bar => 0,
                    ContentDisplayType::Text => 1,
                    ContentDisplayType::Graph => 2,
                    ContentDisplayType::CoreBars => 3,
                    ContentDisplayType::Static => 4,
                    ContentDisplayType::LevelBar => 0, // Fallback to Bar
                })
                .unwrap_or(0)
        };
        type_dropdown.set_selected(current_type);
        type_dropdown.set_hexpand(true);
        type_box.append(&type_dropdown);
        page.append(&type_box);

        // Config container that will hold type-specific widgets
        let config_container = GtkBox::new(Orientation::Vertical, 8);
        page.append(&config_container);

        // Build initial config widgets
        Self::build_content_type_config(
            config,
            on_change,
            preview,
            &slot_name_owned,
            &config_container,
            current_type,
            available_fields,
        );

        // Handle type change
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let slot_name_clone = slot_name_owned.clone();
        let container_clone = config_container.clone();
        let fields_clone = available_fields.clone();
        type_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();

            // Update config
            {
                let mut cfg = config_clone.borrow_mut();
                let item = cfg.frame.content_items.entry(slot_name_clone.clone()).or_default();
                item.display_as = match selected {
                    0 => ContentDisplayType::Bar,
                    1 => ContentDisplayType::Text,
                    2 => ContentDisplayType::Graph,
                    3 => ContentDisplayType::CoreBars,
                    4 => ContentDisplayType::Static,
                    _ => ContentDisplayType::Bar,
                };
            }

            // Rebuild config widgets
            Self::build_content_type_config(
                &config_clone,
                &on_change_clone,
                &preview_clone,
                &slot_name_clone,
                &container_clone,
                selected,
                &fields_clone,
            );

            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        page
    }

    fn build_content_type_config(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        slot_name: &str,
        container: &GtkBox,
        display_type: u32,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
    ) {
        // Clear existing widgets
        while let Some(child) = container.first_child() {
            container.remove(&child);
        }

        let slot_name = slot_name.to_string();

        match display_type {
            0 | 1 => {
                // Bar or Text - use BarConfigWidget
                let bar_config = {
                    let cfg = config.borrow();
                    cfg.frame.content_items.get(&slot_name)
                        .map(|item| item.bar_config.clone())
                        .unwrap_or_default()
                };

                let fields = available_fields.borrow().clone();
                let bar_widget = BarConfigWidget::new(fields);
                bar_widget.set_config(bar_config);

                let on_change_clone = on_change.clone();
                let preview_clone = preview.clone();
                bar_widget.set_on_change(move || {
                    Self::queue_redraw(&preview_clone, &on_change_clone);
                });

                container.append(bar_widget.widget());

                // Note: Config is retrieved from widget when saving via get_config()
            }
            2 => {
                // Graph - use GraphConfigWidget
                let graph_config = {
                    let cfg = config.borrow();
                    cfg.frame.content_items.get(&slot_name)
                        .map(|item| item.graph_config.clone())
                        .unwrap_or_default()
                };

                let fields = available_fields.borrow().clone();
                let graph_widget = GraphConfigWidget::new(fields);
                graph_widget.set_config(graph_config);

                let on_change_clone = on_change.clone();
                let preview_clone = preview.clone();
                graph_widget.set_on_change(move || {
                    Self::queue_redraw(&preview_clone, &on_change_clone);
                });

                container.append(graph_widget.widget());

                // Item height spinner
                let height_box = GtkBox::new(Orientation::Horizontal, 6);
                height_box.append(&Label::new(Some("Item Height:")));
                let height_spin = SpinButton::with_range(50.0, 300.0, 10.0);
                let current_height = {
                    let cfg = config.borrow();
                    cfg.frame.content_items.get(&slot_name)
                        .map(|item| item.item_height)
                        .unwrap_or(100.0)
                };
                height_spin.set_value(current_height);
                height_spin.set_hexpand(true);
                height_box.append(&height_spin);

                let config_clone = config.clone();
                let on_change_clone = on_change.clone();
                let preview_clone = preview.clone();
                let slot_clone = slot_name.clone();
                height_spin.connect_value_changed(move |spin| {
                    let mut cfg = config_clone.borrow_mut();
                    let item = cfg.frame.content_items.entry(slot_clone.clone()).or_default();
                    item.item_height = spin.value();
                    Self::queue_redraw(&preview_clone, &on_change_clone);
                });
                container.append(&height_box);
            }
            3 => {
                // Core Bars - use CoreBarsConfigWidget
                let core_bars_config = {
                    let cfg = config.borrow();
                    cfg.frame.content_items.get(&slot_name)
                        .map(|item| item.core_bars_config.clone())
                        .unwrap_or_default()
                };

                let core_bars_widget = CoreBarsConfigWidget::new();
                core_bars_widget.set_config(core_bars_config);

                let on_change_clone = on_change.clone();
                let preview_clone = preview.clone();
                core_bars_widget.set_on_change(move || {
                    Self::queue_redraw(&preview_clone, &on_change_clone);
                });

                container.append(core_bars_widget.widget());
            }
            4 => {
                // Static - use BackgroundConfigWidget
                let info = Label::new(Some("Static display shows a fixed background/image."));
                info.set_halign(gtk4::Align::Start);
                info.add_css_class("dim-label");
                container.append(&info);

                // Could add BackgroundConfigWidget here for static config
            }
            _ => {}
        }
    }

    fn create_animation_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        animation_widgets_out: &Rc<RefCell<Option<AnimationWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Enable animation
        let enable_check = CheckButton::with_label("Enable Animation");
        enable_check.set_active(config.borrow().animation_enabled);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        enable_check.connect_toggled(move |check| {
            config_clone.borrow_mut().animation_enabled = check.is_active();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        page.append(&enable_check);

        // Animation speed
        let speed_box = GtkBox::new(Orientation::Horizontal, 6);
        speed_box.append(&Label::new(Some("Animation Speed:")));

        let speed_scale = Scale::with_range(Orientation::Horizontal, 1.0, 20.0, 0.5);
        speed_scale.set_value(config.borrow().animation_speed);
        speed_scale.set_hexpand(true);
        speed_scale.set_draw_value(true);
        speed_box.append(&speed_scale);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        speed_scale.connect_value_changed(move |scale| {
            config_clone.borrow_mut().animation_speed = scale.value();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        page.append(&speed_box);

        // Store widget refs
        *animation_widgets_out.borrow_mut() = Some(AnimationWidgets {
            enable_check,
            speed_scale,
        });

        page
    }

    // Public API

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    pub fn get_config(&self) -> CyberpunkDisplayConfig {
        self.config.borrow().clone()
    }

    pub fn set_config(&self, config: &CyberpunkDisplayConfig) {
        *self.config.borrow_mut() = config.clone();

        // Update Frame widgets
        if let Some(widgets) = self.frame_widgets.borrow().as_ref() {
            widgets.border_width_spin.set_value(config.frame.border_width);
            widgets.border_color_widget.set_color(config.frame.border_color);
            widgets.glow_spin.set_value(config.frame.glow_intensity);
            widgets.corner_style_dropdown.set_selected(match config.frame.corner_style {
                CornerStyle::Chamfer => 0,
                CornerStyle::Bracket => 1,
                CornerStyle::Angular => 2,
            });
            widgets.corner_size_spin.set_value(config.frame.corner_size);
            widgets.bg_color_widget.set_color(config.frame.background_color);
            widgets.padding_spin.set_value(config.frame.content_padding);
        }

        // Update Effects widgets
        if let Some(widgets) = self.effects_widgets.borrow().as_ref() {
            widgets.show_grid_check.set_active(config.frame.show_grid);
            widgets.grid_color_widget.set_color(config.frame.grid_color);
            widgets.grid_spacing_spin.set_value(config.frame.grid_spacing);
            widgets.show_scanlines_check.set_active(config.frame.show_scanlines);
            widgets.scanline_opacity_spin.set_value(config.frame.scanline_opacity);
            widgets.item_frame_check.set_active(config.frame.item_frame_enabled);
            widgets.item_frame_color_widget.set_color(config.frame.item_frame_color);
            widgets.item_glow_check.set_active(config.frame.item_glow_enabled);
        }

        // Update Header widgets
        if let Some(widgets) = self.header_widgets.borrow().as_ref() {
            widgets.show_header_check.set_active(config.frame.show_header);
            widgets.header_text_entry.set_text(&config.frame.header_text);
            widgets.header_style_dropdown.set_selected(match config.frame.header_style {
                HeaderStyle::Brackets => 0,
                HeaderStyle::Underline => 1,
                HeaderStyle::Box => 2,
                HeaderStyle::None => 3,
            });
            widgets.header_color_widget.set_color(config.frame.header_color);
            widgets.header_font_btn.set_label(&config.frame.header_font);
            widgets.header_font_size_spin.set_value(config.frame.header_font_size);
        }

        // Update Layout widgets
        if let Some(widgets) = self.layout_widgets.borrow().as_ref() {
            widgets.orientation_dropdown.set_selected(match config.frame.split_orientation {
                SplitOrientation::Vertical => 0,
                SplitOrientation::Horizontal => 1,
            });
            widgets.divider_style_dropdown.set_selected(match config.frame.divider_style {
                DividerStyle::Line => 0,
                DividerStyle::Dashed => 1,
                DividerStyle::Glow => 2,
                DividerStyle::Dots => 3,
                DividerStyle::None => 4,
            });
            widgets.divider_color_widget.set_color(config.frame.divider_color);
            widgets.divider_width_spin.set_value(config.frame.divider_width);

            // Rebuild group weight spinners
            Self::rebuild_group_spinners(
                &self.config,
                &self.on_change,
                &self.preview,
                &widgets.group_weights_box,
            );
        }

        // Update Animation widgets
        if let Some(widgets) = self.animation_widgets.borrow().as_ref() {
            widgets.enable_check.set_active(config.animation_enabled);
            widgets.speed_scale.set_value(config.animation_speed);
        }

        // Rebuild content tabs
        Self::rebuild_content_tabs(
            &self.config,
            &self.on_change,
            &self.preview,
            &self.content_notebook,
            &self.source_summaries,
            &self.available_fields,
        );

        self.preview.queue_draw();
    }

    pub fn set_source_summaries(&self, summaries: Vec<(String, String, usize, u32)>) {
        *self.source_summaries.borrow_mut() = summaries;
        Self::rebuild_content_tabs(
            &self.config,
            &self.on_change,
            &self.preview,
            &self.content_notebook,
            &self.source_summaries,
            &self.available_fields,
        );
    }

    pub fn set_available_fields(&self, fields: Vec<FieldMetadata>) {
        *self.available_fields.borrow_mut() = fields;
    }
}

impl Default for CyberpunkConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
