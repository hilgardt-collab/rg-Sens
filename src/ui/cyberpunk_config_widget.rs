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
use crate::ui::background_config_widget::BackgroundConfigWidget;
use crate::ui::text_line_config_widget::TextLineConfigWidget;
use crate::ui::arc_config_widget::ArcConfigWidget;
use crate::ui::speedometer_config_widget::SpeedometerConfigWidget;
use crate::ui::lcars_display::{ContentDisplayType, ContentItemConfig, SplitOrientation, StaticDisplayConfig};
use crate::displayers::CyberpunkDisplayConfig;
use crate::core::{FieldMetadata, FieldType, FieldPurpose};
use crate::ui::combo_config_base;
use crate::ui::theme::{ColorSource, FontSource};

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
    divider_padding_spin: SpinButton,
    group_weights_box: GtkBox,
}

/// Holds references to Animation tab widgets
struct AnimationWidgets {
    enable_check: CheckButton,
    speed_scale: Scale,
}

/// Holds references to Theme tab widgets
#[allow(dead_code)]
struct ThemeWidgets {
    theme_color1_widget: Rc<ColorButtonWidget>,
    theme_color2_widget: Rc<ColorButtonWidget>,
    theme_color3_widget: Rc<ColorButtonWidget>,
    theme_color4_widget: Rc<ColorButtonWidget>,
    theme_gradient_editor: Rc<crate::ui::gradient_editor::GradientEditor>,
    font1_btn: Button,
    font1_size_spin: SpinButton,
    font2_btn: Button,
    font2_size_spin: SpinButton,
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
    /// Theme tab widgets
    #[allow(dead_code)]
    theme_widgets: Rc<RefCell<Option<ThemeWidgets>>>,
    /// Callbacks to refresh theme reference sections
    #[allow(dead_code)] // Kept for Rc ownership; callbacks are invoked via clones
    theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
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
        let theme_widgets: Rc<RefCell<Option<ThemeWidgets>>> = Rc::new(RefCell::new(None));
        let theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(Vec::new()));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(200);
        preview.set_hexpand(true);
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

        // Theme reference section - placed under preview for easy access from all tabs
        let (theme_ref_section, main_theme_refresh_cb) = combo_config_base::create_theme_reference_section(
            &config,
            |cfg| cfg.frame.theme.clone(),
        );
        theme_ref_refreshers.borrow_mut().push(main_theme_refresh_cb);

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

        // Tab 5: Theme
        let theme_page = Self::create_theme_page(&config, &on_change, &preview, &theme_widgets, &theme_ref_refreshers);
        notebook.append_page(&theme_page, Some(&Label::new(Some("Theme"))));

        // Tab 6: Content - with dynamic per-slot notebook
        let content_notebook = Rc::new(RefCell::new(Notebook::new()));
        let content_page = Self::create_content_page(&config, &on_change, &preview, &content_notebook, &source_summaries, &available_fields, &theme_ref_refreshers);
        notebook.append_page(&content_page, Some(&Label::new(Some("Content"))));

        // Tab 7: Animation
        let animation_page = Self::create_animation_page(&config, &on_change, &animation_widgets);
        notebook.append_page(&animation_page, Some(&Label::new(Some("Animation"))));

        container.append(&preview);
        container.append(&theme_ref_section);
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
            theme_widgets,
            theme_ref_refreshers,
        }
    }

    fn set_page_margins(page: &GtkBox) {
        combo_config_base::set_page_margins(page);
    }

    fn queue_redraw(
        preview: &DrawingArea,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
    ) {
        combo_config_base::queue_redraw(preview, on_change);
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
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.corner_style = match selected {
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
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.header_style = match selected {
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
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.split_orientation = match selected {
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
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            config_clone.borrow_mut().frame.divider_style = match selected {
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

        // Divider padding
        let div_padding_box = GtkBox::new(Orientation::Horizontal, 6);
        div_padding_box.append(&Label::new(Some("Padding:")));
        let divider_padding_spin = SpinButton::with_range(0.0, 20.0, 1.0);
        divider_padding_spin.set_value(config.borrow().frame.divider_padding);
        divider_padding_spin.set_hexpand(true);
        div_padding_box.append(&divider_padding_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        divider_padding_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.divider_padding = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&div_padding_box);

        // Initial build of group weight spinners
        Self::rebuild_group_spinners(config, on_change, preview, &group_weights_box);

        // Store widget refs
        *layout_widgets_out.borrow_mut() = Some(LayoutWidgets {
            orientation_dropdown,
            divider_style_dropdown,
            divider_color_widget,
            divider_width_spin,
            divider_padding_spin,
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
                drop(cfg);
                Self::queue_redraw(&preview_clone, &on_change_clone);
            });

            weights_box.append(&row);
        }
    }

    /// Helper function to refresh all theme reference sections
    fn refresh_theme_refs(refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>) {
        combo_config_base::refresh_theme_refs(refreshers);
    }

    /// Create the Theme configuration page
    fn create_theme_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        theme_widgets_out: &Rc<RefCell<Option<ThemeWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        use crate::ui::gradient_editor::GradientEditor;

        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        let scroll = ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let inner_box = GtkBox::new(Orientation::Vertical, 8);

        // Info label
        let info_label = Label::new(Some("Configure theme colors, gradient, and fonts.\nThese can be referenced in content items for consistent styling."));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("dim-label");
        info_label.set_wrap(true);
        inner_box.append(&info_label);

        // Theme Colors section
        let colors_frame = gtk4::Frame::new(Some("Theme Colors"));
        let colors_box = GtkBox::new(Orientation::Vertical, 6);
        colors_box.set_margin_start(8);
        colors_box.set_margin_end(8);
        colors_box.set_margin_top(8);
        colors_box.set_margin_bottom(8);

        let color_labels = ["Color 1 (Primary):", "Color 2 (Secondary):", "Color 3 (Accent):", "Color 4 (Highlight):"];
        let mut color_widgets: Vec<Rc<ColorButtonWidget>> = Vec::new();

        for (i, label_text) in color_labels.iter().enumerate() {
            let row = GtkBox::new(Orientation::Horizontal, 8);
            row.append(&Label::new(Some(label_text)));

            let color = match i {
                0 => config.borrow().frame.theme.color1,
                1 => config.borrow().frame.theme.color2,
                2 => config.borrow().frame.theme.color3,
                _ => config.borrow().frame.theme.color4,
            };

            let color_widget = Rc::new(ColorButtonWidget::new(color));
            color_widget.widget().set_hexpand(true);

            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            let preview_clone = preview.clone();
            let refreshers_clone = theme_ref_refreshers.clone();
            let color_idx = i;
            color_widget.set_on_change(move |new_color| {
                let mut cfg = config_clone.borrow_mut();
                match color_idx {
                    0 => cfg.frame.theme.color1 = new_color,
                    1 => cfg.frame.theme.color2 = new_color,
                    2 => cfg.frame.theme.color3 = new_color,
                    _ => cfg.frame.theme.color4 = new_color,
                }
                drop(cfg);
                Self::refresh_theme_refs(&refreshers_clone);
                Self::queue_redraw(&preview_clone, &on_change_clone);
            });

            row.append(color_widget.widget());
            color_widgets.push(color_widget);
            colors_box.append(&row);
        }

        colors_frame.set_child(Some(&colors_box));
        inner_box.append(&colors_frame);

        // Theme Gradient section
        let gradient_frame = gtk4::Frame::new(Some("Theme Gradient"));
        let gradient_box = GtkBox::new(Orientation::Vertical, 6);
        gradient_box.set_margin_start(8);
        gradient_box.set_margin_end(8);
        gradient_box.set_margin_top(8);
        gradient_box.set_margin_bottom(8);

        let gradient_editor = Rc::new(GradientEditor::new());
        gradient_editor.set_gradient_source_config(&config.borrow().frame.theme.gradient);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let refreshers_clone = theme_ref_refreshers.clone();
        let gradient_editor_for_cb = gradient_editor.clone();
        gradient_editor.set_on_change(move || {
            let gradient_config = gradient_editor_for_cb.get_gradient_source_config();
            config_clone.borrow_mut().frame.theme.gradient = gradient_config;
            Self::refresh_theme_refs(&refreshers_clone);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        gradient_box.append(gradient_editor.widget());
        gradient_frame.set_child(Some(&gradient_box));
        inner_box.append(&gradient_frame);

        // Theme Fonts section
        let fonts_frame = gtk4::Frame::new(Some("Theme Fonts"));
        let fonts_box = GtkBox::new(Orientation::Vertical, 6);
        fonts_box.set_margin_start(8);
        fonts_box.set_margin_end(8);
        fonts_box.set_margin_top(8);
        fonts_box.set_margin_bottom(8);

        // Font 1
        let font1_row = GtkBox::new(Orientation::Horizontal, 8);
        font1_row.append(&Label::new(Some("Font 1 (Headers):")));
        let font1_btn = Button::with_label(&config.borrow().frame.theme.font1_family);
        font1_btn.set_hexpand(true);
        let config_for_font1 = config.clone();
        let on_change_for_font1 = on_change.clone();
        let preview_for_font1 = preview.clone();
        let refreshers_for_font1 = theme_ref_refreshers.clone();
        let font1_btn_clone = font1_btn.clone();
        font1_btn.connect_clicked(move |btn| {
            if let Some(window) = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) {
                let current_font = config_for_font1.borrow().frame.theme.font1_family.clone();
                let font_desc = gtk4::pango::FontDescription::from_string(&current_font);
                let config_c = config_for_font1.clone();
                let on_change_c = on_change_for_font1.clone();
                let preview_c = preview_for_font1.clone();
                let btn_c = font1_btn_clone.clone();
                let refreshers_c = refreshers_for_font1.clone();
                shared_font_dialog().choose_font(
                    Some(&window),
                    Some(&font_desc),
                    gtk4::gio::Cancellable::NONE,
                    move |result| {
                        if let Ok(font_desc) = result {
                            let family = font_desc.family()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "sans-serif".to_string());
                            config_c.borrow_mut().frame.theme.font1_family = family.clone();
                            btn_c.set_label(&family);
                            Self::refresh_theme_refs(&refreshers_c);
                            Self::queue_redraw(&preview_c, &on_change_c);
                        }
                    },
                );
            }
        });
        font1_row.append(&font1_btn);

        font1_row.append(&Label::new(Some("Size:")));
        let font1_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        font1_size_spin.set_value(config.borrow().frame.theme.font1_size);
        let config_for_font1_size = config.clone();
        let on_change_for_font1_size = on_change.clone();
        let preview_for_font1_size = preview.clone();
        let refreshers_for_font1_size = theme_ref_refreshers.clone();
        font1_size_spin.connect_value_changed(move |spin| {
            config_for_font1_size.borrow_mut().frame.theme.font1_size = spin.value();
            Self::refresh_theme_refs(&refreshers_for_font1_size);
            Self::queue_redraw(&preview_for_font1_size, &on_change_for_font1_size);
        });
        font1_row.append(&font1_size_spin);
        fonts_box.append(&font1_row);

        // Font 2
        let font2_row = GtkBox::new(Orientation::Horizontal, 8);
        font2_row.append(&Label::new(Some("Font 2 (Content):")));
        let font2_btn = Button::with_label(&config.borrow().frame.theme.font2_family);
        font2_btn.set_hexpand(true);
        let config_for_font2 = config.clone();
        let on_change_for_font2 = on_change.clone();
        let preview_for_font2 = preview.clone();
        let refreshers_for_font2 = theme_ref_refreshers.clone();
        let font2_btn_clone = font2_btn.clone();
        font2_btn.connect_clicked(move |btn| {
            if let Some(window) = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) {
                let current_font = config_for_font2.borrow().frame.theme.font2_family.clone();
                let font_desc = gtk4::pango::FontDescription::from_string(&current_font);
                let config_c = config_for_font2.clone();
                let on_change_c = on_change_for_font2.clone();
                let preview_c = preview_for_font2.clone();
                let btn_c = font2_btn_clone.clone();
                let refreshers_c = refreshers_for_font2.clone();
                shared_font_dialog().choose_font(
                    Some(&window),
                    Some(&font_desc),
                    gtk4::gio::Cancellable::NONE,
                    move |result| {
                        if let Ok(font_desc) = result {
                            let family = font_desc.family()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "sans-serif".to_string());
                            config_c.borrow_mut().frame.theme.font2_family = family.clone();
                            btn_c.set_label(&family);
                            Self::refresh_theme_refs(&refreshers_c);
                            Self::queue_redraw(&preview_c, &on_change_c);
                        }
                    },
                );
            }
        });
        font2_row.append(&font2_btn);

        font2_row.append(&Label::new(Some("Size:")));
        let font2_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        font2_size_spin.set_value(config.borrow().frame.theme.font2_size);
        let config_for_font2_size = config.clone();
        let on_change_for_font2_size = on_change.clone();
        let preview_for_font2_size = preview.clone();
        let refreshers_for_font2_size = theme_ref_refreshers.clone();
        font2_size_spin.connect_value_changed(move |spin| {
            config_for_font2_size.borrow_mut().frame.theme.font2_size = spin.value();
            Self::refresh_theme_refs(&refreshers_for_font2_size);
            Self::queue_redraw(&preview_for_font2_size, &on_change_for_font2_size);
        });
        font2_row.append(&font2_size_spin);
        fonts_box.append(&font2_row);

        fonts_frame.set_child(Some(&fonts_box));
        inner_box.append(&fonts_frame);

        // Color Scheme presets section
        let preset_frame = gtk4::Frame::new(Some("Color Scheme Presets"));
        let preset_box = GtkBox::new(Orientation::Vertical, 6);
        preset_box.set_margin_start(8);
        preset_box.set_margin_end(8);
        preset_box.set_margin_top(8);
        preset_box.set_margin_bottom(8);

        let preset_row = GtkBox::new(Orientation::Horizontal, 8);
        preset_row.append(&Label::new(Some("Apply Preset:")));
        let preset_list = StringList::new(&[
            "Cyberpunk (Default)",
            "Synthwave",
            "LCARS",
            "Industrial",
            "Material",
            "Retro Terminal",
            "Fighter HUD",
        ]);
        let preset_dropdown = DropDown::new(Some(preset_list), None::<gtk4::Expression>);
        preset_dropdown.set_hexpand(true);
        preset_dropdown.set_selected(gtk4::INVALID_LIST_POSITION);

        let config_for_preset = config.clone();
        let on_change_for_preset = on_change.clone();
        let preview_for_preset = preview.clone();
        let refreshers_for_preset = theme_ref_refreshers.clone();
        let color_widgets_clone = color_widgets.clone();
        let gradient_editor_for_preset = gradient_editor.clone();
        let font1_btn_for_preset = font1_btn.clone();
        let font1_size_spin_for_preset = font1_size_spin.clone();
        let font2_btn_for_preset = font2_btn.clone();
        let font2_size_spin_for_preset = font2_size_spin.clone();
        preset_dropdown.connect_selected_notify(move |dropdown| {
            use crate::ui::theme::ComboThemeConfig;
            let theme = match dropdown.selected() {
                0 => ComboThemeConfig::default_for_cyberpunk(),
                1 => ComboThemeConfig::default_for_synthwave(),
                2 => ComboThemeConfig::default_for_lcars(),
                3 => ComboThemeConfig::default_for_industrial(),
                4 => ComboThemeConfig::default_for_material(),
                5 => ComboThemeConfig::default_for_retro_terminal(),
                6 => ComboThemeConfig::default_for_fighter_hud(),
                _ => return,
            };
            config_for_preset.borrow_mut().frame.theme = theme.clone();
            // Update UI widgets
            if color_widgets_clone.len() >= 4 {
                color_widgets_clone[0].set_color(theme.color1);
                color_widgets_clone[1].set_color(theme.color2);
                color_widgets_clone[2].set_color(theme.color3);
                color_widgets_clone[3].set_color(theme.color4);
            }
            gradient_editor_for_preset.set_gradient_source_config(&theme.gradient);
            font1_btn_for_preset.set_label(&theme.font1_family);
            font1_size_spin_for_preset.set_value(theme.font1_size);
            font2_btn_for_preset.set_label(&theme.font2_family);
            font2_size_spin_for_preset.set_value(theme.font2_size);
            Self::refresh_theme_refs(&refreshers_for_preset);
            Self::queue_redraw(&preview_for_preset, &on_change_for_preset);
        });
        preset_row.append(&preset_dropdown);
        preset_box.append(&preset_row);
        preset_frame.set_child(Some(&preset_box));
        inner_box.append(&preset_frame);

        scroll.set_child(Some(&inner_box));
        page.append(&scroll);

        // Store theme widgets for later updates
        *theme_widgets_out.borrow_mut() = Some(ThemeWidgets {
            theme_color1_widget: color_widgets[0].clone(),
            theme_color2_widget: color_widgets[1].clone(),
            theme_color3_widget: color_widgets[2].clone(),
            theme_color4_widget: color_widgets[3].clone(),
            theme_gradient_editor: gradient_editor,
            font1_btn: font1_btn.clone(),
            font1_size_spin: font1_size_spin.clone(),
            font2_btn: font2_btn.clone(),
            font2_size_spin: font2_size_spin.clone(),
        });

        page
    }

    /// Create a theme reference section showing current theme colors and fonts with copy buttons
    #[allow(dead_code)]
    fn create_theme_reference_section(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
    ) -> (gtk4::Frame, Rc<dyn Fn()>) {
        use crate::ui::clipboard::CLIPBOARD;

        let frame = gtk4::Frame::new(Some("Theme Reference"));
        frame.set_margin_top(8);

        let content_box = GtkBox::new(Orientation::Vertical, 6);
        content_box.set_margin_start(8);
        content_box.set_margin_end(8);
        content_box.set_margin_top(8);
        content_box.set_margin_bottom(8);

        // Colors row
        let colors_box = GtkBox::new(Orientation::Horizontal, 8);
        colors_box.append(&Label::new(Some("Colors:")));

        // Store swatches for refresh
        let color_swatches: Rc<RefCell<Vec<DrawingArea>>> = Rc::new(RefCell::new(Vec::new()));

        let color_indices = [1u8, 2, 3, 4];
        let color_tooltips = ["Color 1 (Primary)", "Color 2 (Secondary)", "Color 3 (Accent)", "Color 4 (Highlight)"];

        for (idx, tooltip) in color_indices.iter().zip(color_tooltips.iter()) {
            let item_box = GtkBox::new(Orientation::Horizontal, 2);

            let swatch = DrawingArea::new();
            swatch.set_size_request(20, 20);
            let config_for_draw = config.clone();
            let color_idx = *idx;
            swatch.set_draw_func(move |_, cr, width, height| {
                let c = config_for_draw.borrow().frame.theme.get_color(color_idx);
                let checker_size = 4.0;
                for y in 0..(height as f64 / checker_size).ceil() as i32 {
                    for x in 0..(width as f64 / checker_size).ceil() as i32 {
                        if (x + y) % 2 == 0 {
                            cr.set_source_rgb(0.8, 0.8, 0.8);
                        } else {
                            cr.set_source_rgb(0.6, 0.6, 0.6);
                        }
                        cr.rectangle(x as f64 * checker_size, y as f64 * checker_size, checker_size, checker_size);
                        let _ = cr.fill();
                    }
                }
                cr.set_source_rgba(c.r, c.g, c.b, c.a);
                cr.rectangle(0.0, 0.0, width as f64, height as f64);
                let _ = cr.fill();
                cr.set_source_rgb(0.3, 0.3, 0.3);
                cr.set_line_width(1.0);
                cr.rectangle(0.5, 0.5, width as f64 - 1.0, height as f64 - 1.0);
                let _ = cr.stroke();
            });
            color_swatches.borrow_mut().push(swatch.clone());
            item_box.append(&swatch);

            let copy_btn = Button::from_icon_name("edit-copy-symbolic");
            copy_btn.set_tooltip_text(Some(&format!("Copy {} to clipboard", tooltip)));
            let config_for_copy = config.clone();
            let color_idx_for_copy = *idx;
            copy_btn.connect_clicked(move |_| {
                let c = config_for_copy.borrow().frame.theme.get_color(color_idx_for_copy);
                if let Ok(mut clipboard) = CLIPBOARD.lock() {
                    clipboard.copy_color(c.r, c.g, c.b, c.a);
                }
            });
            item_box.append(&copy_btn);
            colors_box.append(&item_box);
        }
        content_box.append(&colors_box);

        // Gradient row
        let gradient_box = GtkBox::new(Orientation::Horizontal, 8);
        gradient_box.append(&Label::new(Some("Gradient:")));

        let gradient_swatch = DrawingArea::new();
        gradient_swatch.set_size_request(60, 20);
        let config_for_gradient = config.clone();
        gradient_swatch.set_draw_func(move |_, cr, width, height| {
            let cfg = config_for_gradient.borrow();
            let gradient_config = cfg.frame.theme.gradient.resolve(&cfg.frame.theme);
            let w = width as f64;
            let h = height as f64;
            let angle_rad = gradient_config.angle.to_radians();
            let (dx, dy) = (angle_rad.sin(), -angle_rad.cos());
            let length = (w * dx.abs() + h * dy.abs()) / 2.0;
            let (cx, cy) = (w / 2.0, h / 2.0);
            let (x0, y0) = (cx - dx * length, cy - dy * length);
            let (x1, y1) = (cx + dx * length, cy + dy * length);
            let gradient = gtk4::cairo::LinearGradient::new(x0, y0, x1, y1);
            for stop in &gradient_config.stops {
                gradient.add_color_stop_rgba(stop.position, stop.color.r, stop.color.g, stop.color.b, stop.color.a);
            }
            let _ = cr.set_source(&gradient);
            cr.rectangle(0.0, 0.0, w, h);
            let _ = cr.fill();
            cr.set_source_rgb(0.3, 0.3, 0.3);
            cr.set_line_width(1.0);
            cr.rectangle(0.5, 0.5, w - 1.0, h - 1.0);
            let _ = cr.stroke();
        });
        gradient_box.append(&gradient_swatch);

        let gradient_copy_btn = Button::from_icon_name("edit-copy-symbolic");
        gradient_copy_btn.set_tooltip_text(Some("Copy Theme Gradient to clipboard"));
        let config_for_gradient_copy = config.clone();
        gradient_copy_btn.connect_clicked(move |_| {
            let cfg = config_for_gradient_copy.borrow();
            let resolved_gradient = cfg.frame.theme.gradient.resolve(&cfg.frame.theme);
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_gradient_stops(resolved_gradient.stops);
            }
        });
        gradient_box.append(&gradient_copy_btn);
        content_box.append(&gradient_box);

        // Fonts row
        let fonts_box = GtkBox::new(Orientation::Horizontal, 8);
        fonts_box.append(&Label::new(Some("Fonts:")));

        let font_labels: Rc<RefCell<Vec<Label>>> = Rc::new(RefCell::new(Vec::new()));
        let font_indices = [1u8, 2];
        let font_tooltips = ["Font 1 (Headers)", "Font 2 (Content)"];

        for (idx, tooltip) in font_indices.iter().zip(font_tooltips.iter()) {
            let item_box = GtkBox::new(Orientation::Horizontal, 4);
            let (family, size) = config.borrow().frame.theme.get_font(*idx);
            let info = Label::new(Some(&format!("{} {}pt", family, size as i32)));
            info.add_css_class("dim-label");
            font_labels.borrow_mut().push(info.clone());
            item_box.append(&info);

            let copy_btn = Button::from_icon_name("edit-copy-symbolic");
            copy_btn.set_tooltip_text(Some(&format!("Copy {} to clipboard", tooltip)));
            let font_idx = *idx;
            copy_btn.connect_clicked(move |_| {
                if let Ok(mut clipboard) = CLIPBOARD.lock() {
                    clipboard.copy_font_source(FontSource::Theme { index: font_idx }, false, false);
                }
            });
            item_box.append(&copy_btn);
            fonts_box.append(&item_box);
        }
        content_box.append(&fonts_box);
        frame.set_child(Some(&content_box));

        let config_for_refresh = config.clone();
        let gradient_swatch_for_refresh = gradient_swatch.clone();
        let refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            for swatch in color_swatches.borrow().iter() {
                swatch.queue_draw();
            }
            gradient_swatch_for_refresh.queue_draw();
            let cfg = config_for_refresh.borrow();
            let labels = font_labels.borrow();
            if labels.len() >= 2 {
                let (family1, size1) = cfg.frame.theme.get_font(1);
                labels[0].set_text(&format!("{} {}pt", family1, size1 as i32));
                let (family2, size2) = cfg.frame.theme.get_font(2);
                labels[1].set_text(&format!("{} {}pt", family2, size2 as i32));
            }
        });

        (frame, refresh_callback)
    }

    fn create_content_page(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
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
        Self::rebuild_content_tabs(config, on_change, preview, content_notebook, source_summaries, available_fields, theme_ref_refreshers);

        page
    }

    fn rebuild_content_tabs(
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) {
        let notebook = content_notebook.borrow();

        // Clear existing tabs
        while notebook.n_pages() > 0 {
            notebook.remove_page(Some(0));
        }

        let summaries = source_summaries.borrow();

        log::info!(
            "=== Cyberpunk rebuild_content_tabs: source_summaries has {} entries ===",
            summaries.len()
        );

        if summaries.is_empty() {
            // No sources configured yet - show placeholder
            log::warn!("rebuild_content_tabs: summaries is EMPTY, showing placeholder");
            let placeholder = GtkBox::new(Orientation::Vertical, 8);
            placeholder.set_margin_start(12);
            placeholder.set_margin_end(12);
            placeholder.set_margin_top(12);
            let label = Label::new(Some("No sources configured.\nGo to 'Data Source' tab and select 'Combination' source to configure content."));
            label.set_halign(gtk4::Align::Start);
            placeholder.append(&label);
            notebook.append_page(&placeholder, Some(&Label::new(Some("No Sources"))));
            return;
        }

        // Group summaries by group number
        let mut groups: std::collections::HashMap<usize, Vec<(String, String, u32)>> = std::collections::HashMap::new();
        for (slot_name, summary, group_num, item_idx) in summaries.iter() {
            groups.entry(*group_num)
                .or_default()
                .push((slot_name.clone(), summary.clone(), *item_idx));
        }

        // Sort groups by group number
        let mut group_nums: Vec<usize> = groups.keys().cloned().collect();
        group_nums.sort();

        // Create a tab for each group (with nested notebook for items)
        for group_num in group_nums {
            if let Some(items) = groups.get(&group_num) {
                let group_box = GtkBox::new(Orientation::Vertical, 4);
                group_box.set_margin_start(4);
                group_box.set_margin_end(4);
                group_box.set_margin_top(4);

                // Nested notebook for items in this group
                let items_notebook = Notebook::new();
                items_notebook.set_scrollable(true);
                items_notebook.set_vexpand(true);

                // Sort items by item index
                let mut sorted_items = items.clone();
                sorted_items.sort_by_key(|(_, _, idx)| *idx);

                for (slot_name, summary, item_idx) in sorted_items {
                    let tab_label = format!("Item {} : {}", item_idx, summary);
                    let tab_box = Self::create_slot_config_tab(&slot_name, config, on_change, preview, available_fields, theme_ref_refreshers);
                    items_notebook.append_page(&tab_box, Some(&Label::new(Some(&tab_label))));
                }

                group_box.append(&items_notebook);
                notebook.append_page(&group_box, Some(&Label::new(Some(&format!("Group {}", group_num)))));
            }
        }
    }

    fn create_slot_config_tab(
        slot_name: &str,
        config: &Rc<RefCell<CyberpunkDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        log::info!("=== Cyberpunk create_slot_config_tab() called for slot '{}' ===", slot_name);

        // Get available fields for this slot (needed for smart defaults)
        let slot_prefix = format!("{}_", slot_name);
        let slot_fields_for_default: Vec<FieldMetadata> = available_fields.borrow().iter()
            .filter(|f| f.id.starts_with(&slot_prefix))
            .map(|f| {
                let short_id = f.id.strip_prefix(&slot_prefix).unwrap_or(&f.id);
                FieldMetadata::new(
                    short_id,
                    &f.name,
                    &f.description,
                    f.field_type.clone(),
                    f.purpose.clone(),
                )
            })
            .collect();

        // Ensure this slot exists in content_items with smart default display type
        // This is critical for newly added items to be saved properly
        {
            let mut cfg = config.borrow_mut();
            if !cfg.frame.content_items.contains_key(slot_name) {
                log::info!("Creating default content item for new slot '{}'", slot_name);
                let item = ContentItemConfig {
                    display_as: ContentDisplayType::suggest_for_fields(&slot_fields_for_default),
                    ..Default::default()
                };
                cfg.frame.content_items.insert(slot_name.to_string(), item);
            }
        }

        let tab = GtkBox::new(Orientation::Vertical, 8);
        tab.set_margin_start(12);
        tab.set_margin_end(12);
        tab.set_margin_top(12);
        tab.set_margin_bottom(12);

        // Make it scrollable for small screens
        let scroll = ScrolledWindow::new();
        scroll.set_vexpand(true);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let inner_box = GtkBox::new(Orientation::Vertical, 8);

        // Display type dropdown
        let type_box = GtkBox::new(Orientation::Horizontal, 6);
        type_box.append(&Label::new(Some("Display As:")));
        let type_list = StringList::new(&["Bar", "Text", "Graph", "Core Bars", "Static", "Arc", "Speedometer"]);
        let type_dropdown = DropDown::new(Some(type_list), None::<gtk4::Expression>);
        type_dropdown.set_hexpand(true);

        // Get current display type for this slot
        let current_type = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.display_as)
                .unwrap_or_default()
        };
        let type_idx = match current_type {
            ContentDisplayType::Bar | ContentDisplayType::LevelBar => 0,
            ContentDisplayType::Text => 1,
            ContentDisplayType::Graph => 2,
            ContentDisplayType::CoreBars => 3,
            ContentDisplayType::Static => 4,
            ContentDisplayType::Arc => 5,
            ContentDisplayType::Speedometer => 6,
        };
        type_dropdown.set_selected(type_idx);
        type_box.append(&type_dropdown);
        inner_box.append(&type_box);

        // Auto height checkbox
        let auto_height_check = CheckButton::with_label("Auto-adjust height");
        let current_auto_height = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.auto_height)
                .unwrap_or(true)
        };
        auto_height_check.set_active(current_auto_height);
        inner_box.append(&auto_height_check);

        // Item height
        let height_box = GtkBox::new(Orientation::Horizontal, 6);
        height_box.append(&Label::new(Some("Item Height:")));
        let height_spin = SpinButton::with_range(20.0, 300.0, 5.0);
        let current_height = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.item_height)
                .unwrap_or(60.0)
        };
        height_spin.set_value(current_height);
        height_spin.set_hexpand(true);
        height_spin.set_sensitive(!current_auto_height);
        height_box.append(&height_spin);
        inner_box.append(&height_box);

        // Connect auto-height checkbox to control spinner sensitivity
        let height_spin_clone = height_spin.clone();
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        auto_height_check.connect_toggled(move |check| {
            let is_auto = check.is_active();
            height_spin_clone.set_sensitive(!is_auto);
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.auto_height = is_auto;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Get available fields - filter to fields relevant to this slot
        let slot_prefix = format!("{}_", slot_name);
        let source_fields = available_fields.borrow();
        let mut slot_fields: Vec<FieldMetadata> = source_fields.iter()
            .filter(|f| f.id.starts_with(&slot_prefix))
            .map(|f| {
                // Remove the slot prefix for display in the dropdown
                let short_id = f.id.strip_prefix(&slot_prefix).unwrap_or(&f.id);
                FieldMetadata::new(
                    short_id,
                    &f.name,
                    &f.description,
                    f.field_type.clone(),
                    f.purpose.clone(),
                )
            })
            .collect();

        // If no slot-specific fields found, add generic fallback fields
        if slot_fields.is_empty() {
            slot_fields = vec![
                FieldMetadata::new("caption", "Caption", "Label text for the item", FieldType::Text, FieldPurpose::Caption),
                FieldMetadata::new("value", "Value", "Current value with formatting", FieldType::Text, FieldPurpose::Value),
                FieldMetadata::new("unit", "Unit", "Unit of measurement", FieldType::Text, FieldPurpose::Unit),
                FieldMetadata::new("numerical_value", "Numeric Value", "Raw numeric value", FieldType::Numerical, FieldPurpose::Value),
                FieldMetadata::new("min_value", "Minimum", "Minimum value for range", FieldType::Numerical, FieldPurpose::Other),
                FieldMetadata::new("max_value", "Maximum", "Maximum value for range", FieldType::Numerical, FieldPurpose::Other),
            ];
        }
        drop(source_fields);

        // === Bar Configuration Section ===
        let bar_config_frame = gtk4::Frame::new(Some("Bar Configuration"));
        bar_config_frame.set_margin_top(12);

        let bar_widget = BarConfigWidget::new(slot_fields.clone());

        // Initialize with current config
        let current_bar_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.bar_config.clone())
                .unwrap_or_else(Self::default_bar_config_cyberpunk)
        };
        bar_widget.set_config(current_bar_config);
        // Set initial theme
        bar_widget.set_theme(config.borrow().frame.theme.clone());

        // Set up change callback to sync config back
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let bar_widget_rc = Rc::new(bar_widget);
        let bar_widget_for_callback = bar_widget_rc.clone();
        bar_widget_rc.set_on_change(move || {
            let bar_config = bar_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.bar_config = bar_config;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Register theme refresh callback for bar widget
        let bar_widget_for_theme = bar_widget_rc.clone();
        let config_for_bar_theme = config.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = config_for_bar_theme.borrow().frame.theme.clone();
            bar_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);

        bar_config_frame.set_child(Some(bar_widget_rc.widget()));
        inner_box.append(&bar_config_frame);

        // === Graph Configuration Section ===
        let graph_config_frame = gtk4::Frame::new(Some("Graph Configuration"));
        graph_config_frame.set_margin_top(12);

        let graph_widget = GraphConfigWidget::new(slot_fields.clone());

        // Initialize with current config
        let current_graph_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.graph_config.clone())
                .unwrap_or_else(Self::default_graph_config_cyberpunk)
        };
        graph_widget.set_config(current_graph_config);

        // Set up change callback
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let graph_widget_rc = Rc::new(graph_widget);
        let graph_widget_for_callback = graph_widget_rc.clone();
        graph_widget_rc.set_on_change(move || {
            let graph_config = graph_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.graph_config = graph_config;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        graph_config_frame.set_child(Some(graph_widget_rc.widget()));
        inner_box.append(&graph_config_frame);

        // === Text Configuration Section ===
        let text_config_frame = gtk4::Frame::new(Some("Text Configuration"));
        text_config_frame.set_margin_top(12);

        let text_widget = TextLineConfigWidget::new(slot_fields.clone());

        // Initialize with current config
        let current_text_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.bar_config.text_overlay.text_config.clone())
                .unwrap_or_default()
        };
        text_widget.set_config(current_text_config);

        // Set up change callback
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let text_widget_rc = Rc::new(text_widget);
        let text_widget_for_callback = text_widget_rc.clone();
        text_widget_rc.set_on_change(move || {
            let text_config = text_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.bar_config.text_overlay.enabled = true;
            item.bar_config.text_overlay.text_config = text_config;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        text_config_frame.set_child(Some(text_widget_rc.widget()));
        inner_box.append(&text_config_frame);

        // === Core Bars Configuration Section ===
        let core_bars_config_frame = gtk4::Frame::new(Some("Core Bars Configuration"));
        core_bars_config_frame.set_margin_top(12);

        let core_bars_widget = CoreBarsConfigWidget::new();

        // Initialize with current config
        let current_core_bars_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.core_bars_config.clone())
                .unwrap_or_else(Self::default_core_bars_config_cyberpunk)
        };
        // Set theme BEFORE config, since set_config triggers UI rebuild that needs theme
        core_bars_widget.set_theme(config.borrow().frame.theme.clone());
        core_bars_widget.set_config(current_core_bars_config);

        // Set up change callback
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let core_bars_widget_rc = Rc::new(core_bars_widget);
        let core_bars_widget_for_callback = core_bars_widget_rc.clone();
        core_bars_widget_rc.set_on_change(move || {
            let core_bars_config = core_bars_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.core_bars_config = core_bars_config;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Register theme refresh callback for core bars widget
        let core_bars_widget_for_theme = core_bars_widget_rc.clone();
        let config_for_core_bars_theme = config.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = config_for_core_bars_theme.borrow().frame.theme.clone();
            core_bars_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);

        core_bars_config_frame.set_child(Some(core_bars_widget_rc.widget()));
        inner_box.append(&core_bars_config_frame);

        // === Static Configuration Section ===
        let static_config_frame = gtk4::Frame::new(Some("Static Background Configuration"));
        static_config_frame.set_margin_top(12);

        let static_bg_widget = BackgroundConfigWidget::new();

        // Initialize with current config
        let current_static_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.static_config.background.clone())
                .unwrap_or_default()
        };
        static_bg_widget.set_config(current_static_config);

        // Set up change callback
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let static_bg_widget_rc = Rc::new(static_bg_widget);
        let static_bg_widget_for_callback = static_bg_widget_rc.clone();
        static_bg_widget_rc.set_on_change(move || {
            let bg_config = static_bg_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.static_config = StaticDisplayConfig { background: bg_config };
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        static_config_frame.set_child(Some(static_bg_widget_rc.widget()));
        inner_box.append(&static_config_frame);

        // === Arc Configuration Section ===
        let arc_config_frame = gtk4::Frame::new(Some("Arc Gauge Configuration"));
        arc_config_frame.set_margin_top(12);

        let arc_widget = ArcConfigWidget::new(slot_fields.clone());

        // Initialize with current config
        let current_arc_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.arc_config.clone())
                .unwrap_or_else(Self::default_arc_config_cyberpunk)
        };
        arc_widget.set_config(current_arc_config);
        // Set initial theme
        arc_widget.set_theme(config.borrow().frame.theme.clone());

        // Set up change callback
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let arc_widget_rc = Rc::new(arc_widget);
        let arc_widget_for_callback = arc_widget_rc.clone();
        arc_widget_rc.set_on_change(move || {
            let arc_config = arc_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.arc_config = arc_config;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Register theme refresh callback for arc widget
        let arc_widget_for_theme = arc_widget_rc.clone();
        let config_for_arc_theme = config.clone();
        let theme_refresh_callback: Rc<dyn Fn()> = Rc::new(move || {
            let theme = config_for_arc_theme.borrow().frame.theme.clone();
            arc_widget_for_theme.set_theme(theme);
        });
        theme_ref_refreshers.borrow_mut().push(theme_refresh_callback);

        arc_config_frame.set_child(Some(arc_widget_rc.widget()));
        inner_box.append(&arc_config_frame);

        // === Speedometer Configuration Section ===
        let speedometer_config_frame = gtk4::Frame::new(Some("Speedometer Configuration"));
        speedometer_config_frame.set_margin_top(12);

        let speedometer_widget = SpeedometerConfigWidget::new(slot_fields.clone());

        // Initialize with current config
        let current_speedometer_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.speedometer_config.clone())
                .unwrap_or_else(Self::default_speedometer_config_cyberpunk)
        };
        speedometer_widget.set_config(&current_speedometer_config);

        // Set up change callback
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let speedometer_widget_rc = Rc::new(speedometer_widget);
        let speedometer_widget_for_callback = speedometer_widget_rc.clone();
        speedometer_widget_rc.set_on_change(Box::new(move || {
            let speedometer_config = speedometer_widget_for_callback.get_config();
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.speedometer_config = speedometer_config;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        }));

        speedometer_config_frame.set_child(Some(speedometer_widget_rc.widget()));
        inner_box.append(&speedometer_config_frame);

        // Show/hide config sections based on display type
        let show_bar = matches!(current_type, ContentDisplayType::Bar | ContentDisplayType::LevelBar);
        let show_text = matches!(current_type, ContentDisplayType::Text | ContentDisplayType::Static);
        bar_config_frame.set_visible(show_bar);
        text_config_frame.set_visible(show_text);
        graph_config_frame.set_visible(current_type == ContentDisplayType::Graph);
        core_bars_config_frame.set_visible(current_type == ContentDisplayType::CoreBars);
        static_config_frame.set_visible(current_type == ContentDisplayType::Static);
        arc_config_frame.set_visible(current_type == ContentDisplayType::Arc);
        speedometer_config_frame.set_visible(current_type == ContentDisplayType::Speedometer);

        scroll.set_child(Some(&inner_box));
        tab.append(&scroll);

        // === Connect change handlers ===

        // Display type change handler (toggles config section visibility)
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let bar_config_frame_clone = bar_config_frame.clone();
        let text_config_frame_clone = text_config_frame.clone();
        let graph_config_frame_clone = graph_config_frame.clone();
        let core_bars_config_frame_clone = core_bars_config_frame.clone();
        let static_config_frame_clone = static_config_frame.clone();
        let arc_config_frame_clone = arc_config_frame.clone();
        let speedometer_config_frame_clone = speedometer_config_frame.clone();
        type_dropdown.connect_selected_notify(move |dropdown| {
            let selected = dropdown.selected();
            if selected == gtk4::INVALID_LIST_POSITION {
                return;
            }
            let display_type = match selected {
                0 => ContentDisplayType::Bar,
                1 => ContentDisplayType::Text,
                2 => ContentDisplayType::Graph,
                3 => ContentDisplayType::CoreBars,
                4 => ContentDisplayType::Static,
                5 => ContentDisplayType::Arc,
                _ => ContentDisplayType::Speedometer,
            };
            // Show appropriate config for each display type
            let show_bar = matches!(display_type, ContentDisplayType::Bar | ContentDisplayType::LevelBar);
            let show_text = matches!(display_type, ContentDisplayType::Text | ContentDisplayType::Static);
            bar_config_frame_clone.set_visible(show_bar);
            text_config_frame_clone.set_visible(show_text);
            graph_config_frame_clone.set_visible(display_type == ContentDisplayType::Graph);
            core_bars_config_frame_clone.set_visible(display_type == ContentDisplayType::CoreBars);
            static_config_frame_clone.set_visible(display_type == ContentDisplayType::Static);
            arc_config_frame_clone.set_visible(display_type == ContentDisplayType::Arc);
            speedometer_config_frame_clone.set_visible(display_type == ContentDisplayType::Speedometer);
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.display_as = display_type;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Item height change handler
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        height_spin.connect_value_changed(move |spin| {
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.item_height = spin.value();
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        tab
    }

    /// Default bar config with cyberpunk colors (cyan/magenta gradient)
    #[allow(clippy::field_reassign_with_default)]
    fn default_bar_config_cyberpunk() -> crate::ui::BarDisplayConfig {
        use crate::ui::bar_display::{BarDisplayConfig, BarStyle, BarOrientation, BarFillDirection, BarFillType, BarBackgroundType, BorderConfig};
        use crate::ui::background::Color;

        let mut config = BarDisplayConfig::default();
        config.style = BarStyle::Full;
        config.orientation = BarOrientation::Horizontal;
        config.fill_direction = BarFillDirection::LeftToRight;

        // Cyberpunk cyan to magenta gradient
        config.foreground = BarFillType::Gradient {
            stops: vec![
                crate::ui::theme::ColorStopSource::custom(0.0, Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 }),  // Cyan
                crate::ui::theme::ColorStopSource::custom(1.0, Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 }),  // Magenta
            ],
            angle: 0.0,
        };
        config.background = BarBackgroundType::Solid {
            color: crate::ui::theme::ColorSource::custom(Color { r: 0.1, g: 0.1, b: 0.15, a: 0.8 })  // Dark background
        };
        config.border = BorderConfig {
            enabled: true,
            color: crate::ui::theme::ColorSource::custom(Color { r: 0.0, g: 1.0, b: 1.0, a: 0.5 }),  // Cyan border
            width: 1.0,
        };

        config
    }

    /// Default graph config with cyberpunk colors
    #[allow(clippy::field_reassign_with_default)]
    fn default_graph_config_cyberpunk() -> crate::ui::GraphDisplayConfig {
        use crate::ui::graph_display::{GraphDisplayConfig, GraphType, LineStyle, FillMode};
        use crate::ui::background::Color;

        let mut config = GraphDisplayConfig::default();
        config.graph_type = GraphType::Line;
        config.line_style = LineStyle::Solid;
        config.line_width = 2.0;
        config.line_color = ColorSource::custom(Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 });  // Cyan
        config.fill_mode = FillMode::Gradient;
        config.fill_gradient_start = ColorSource::custom(Color { r: 0.0, g: 1.0, b: 1.0, a: 0.4 });  // Cyan start
        config.fill_gradient_end = ColorSource::custom(Color { r: 0.0, g: 1.0, b: 1.0, a: 0.0 });    // Transparent end
        config.background_color = Color { r: 0.04, g: 0.06, b: 0.1, a: 0.8 };
        config.plot_background_color = Color { r: 0.04, g: 0.06, b: 0.1, a: 0.6 };
        config.x_axis.show_grid = true;
        config.x_axis.grid_color = ColorSource::custom(Color { r: 0.0, g: 0.5, b: 0.5, a: 0.3 });  // Cyan grid
        config.y_axis.show_grid = true;
        config.y_axis.grid_color = ColorSource::custom(Color { r: 0.0, g: 0.5, b: 0.5, a: 0.3 });  // Cyan grid

        config
    }

    /// Default core bars config with cyberpunk colors
    #[allow(clippy::field_reassign_with_default)]
    fn default_core_bars_config_cyberpunk() -> crate::ui::CoreBarsConfig {
        use crate::ui::core_bars_display::CoreBarsConfig;
        use crate::ui::bar_display::{BarStyle, BarFillType, BarBackgroundType, BorderConfig};
        use crate::ui::background::Color;

        let mut config = CoreBarsConfig::default();
        config.bar_style = BarStyle::Full;

        // Cyberpunk cyan to yellow to magenta gradient
        config.foreground = BarFillType::Gradient {
            stops: vec![
                crate::ui::theme::ColorStopSource::custom(0.0, Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 }),  // Cyan (low)
                crate::ui::theme::ColorStopSource::custom(0.5, Color { r: 1.0, g: 1.0, b: 0.0, a: 1.0 }),  // Yellow (mid)
                crate::ui::theme::ColorStopSource::custom(1.0, Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 }),  // Magenta (high)
            ],
            angle: 90.0,
        };
        config.background = BarBackgroundType::Solid {
            color: crate::ui::theme::ColorSource::custom(Color { r: 0.1, g: 0.1, b: 0.15, a: 0.6 })
        };
        config.border = BorderConfig {
            enabled: true,
            color: crate::ui::theme::ColorSource::custom(Color { r: 0.0, g: 1.0, b: 1.0, a: 0.3 }),
            width: 1.0,
        };

        config
    }

    /// Default arc config with cyberpunk colors
    #[allow(clippy::field_reassign_with_default)]
    fn default_arc_config_cyberpunk() -> crate::ui::ArcDisplayConfig {
        use crate::ui::arc_display::ArcDisplayConfig;
        use crate::ui::background::Color;
        use crate::ui::theme::{ColorSource, ColorStopSource};

        let mut config = ArcDisplayConfig::default();

        // Cyberpunk cyan to magenta gradient
        config.color_stops = vec![
            ColorStopSource { position: 0.0, color: ColorSource::Custom { color: Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 } } },   // Cyan
            ColorStopSource { position: 0.5, color: ColorSource::Custom { color: Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 } } },   // Magenta
            ColorStopSource { position: 1.0, color: ColorSource::Custom { color: Color { r: 1.0, g: 1.0, b: 0.0, a: 1.0 } } },   // Yellow
        ];
        config.show_background_arc = true;
        config.background_color = ColorSource::Custom { color: Color { r: 0.1, g: 0.1, b: 0.15, a: 0.6 } };
        config.animate = true;

        config
    }

    /// Default speedometer config with cyberpunk colors
    #[allow(clippy::field_reassign_with_default)]
    fn default_speedometer_config_cyberpunk() -> crate::ui::SpeedometerConfig {
        use crate::ui::speedometer_display::SpeedometerConfig;
        use crate::ui::background::Color;
        use crate::ui::theme::{ColorSource, ColorStopSource};
        let mut config = SpeedometerConfig::default();

        // Cyberpunk colored track
        config.track_color = ColorSource::Custom { color: Color { r: 0.1, g: 0.1, b: 0.15, a: 0.6 } };
        config.track_color_stops = vec![
            ColorStopSource::custom(0.0, Color { r: 0.0, g: 1.0, b: 1.0, a: 1.0 }),   // Cyan (low)
            ColorStopSource::custom(0.6, Color { r: 1.0, g: 1.0, b: 0.0, a: 1.0 }),   // Yellow (mid)
            ColorStopSource::custom(1.0, Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 }),   // Magenta (high)
        ];

        // Cyberpunk tick colors
        config.major_tick_color = ColorSource::Custom { color: Color { r: 0.0, g: 1.0, b: 1.0, a: 0.8 } };  // Cyan
        config.minor_tick_color = ColorSource::Custom { color: Color { r: 0.0, g: 1.0, b: 1.0, a: 0.4 } };  // Dimmer cyan

        // Cyberpunk needle - magenta with glow effect
        config.needle_color = ColorSource::Custom { color: Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 } };  // Magenta

        // Center hub - dark with cyan accent
        config.center_hub_color = ColorSource::Custom { color: Color { r: 0.0, g: 0.4, b: 0.4, a: 1.0 } };  // Dark cyan
        config.center_hub_3d = true;

        config
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

    /// Set the theme configuration. Call this BEFORE set_config to ensure
    /// font selectors have the correct theme when the UI is rebuilt.
    pub fn set_theme(&self, theme: crate::ui::theme::ComboThemeConfig) {
        self.config.borrow_mut().frame.theme = theme;
        // Trigger all theme refreshers to update child widgets
        for refresher in self.theme_ref_refreshers.borrow().iter() {
            refresher();
        }
    }

    pub fn get_config(&self) -> CyberpunkDisplayConfig {
        self.config.borrow().clone()
    }

    /// Get a reference to the internal config Rc for use in callbacks
    pub fn get_config_rc(&self) -> Rc<RefCell<CyberpunkDisplayConfig>> {
        self.config.clone()
    }

    pub fn set_config(&self, config: &CyberpunkDisplayConfig) {
        log::debug!(
            "CyberpunkConfigWidget::set_config - loading {} groups, {} content_items",
            config.frame.group_count,
            config.frame.content_items.len()
        );
        for (slot_name, item_cfg) in &config.frame.content_items {
            log::debug!("  loading content_item '{}': display_as={:?}", slot_name, item_cfg.display_as);
        }
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
            widgets.divider_padding_spin.set_value(config.frame.divider_padding);

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

        // Update Theme widgets (fonts and colors)
        if let Some(ref widgets) = *self.theme_widgets.borrow() {
            widgets.theme_color1_widget.set_color(config.frame.theme.color1);
            widgets.theme_color2_widget.set_color(config.frame.theme.color2);
            widgets.theme_color3_widget.set_color(config.frame.theme.color3);
            widgets.theme_color4_widget.set_color(config.frame.theme.color4);
            widgets.theme_gradient_editor.set_gradient_source_config(&config.frame.theme.gradient);
            widgets.font1_btn.set_label(&config.frame.theme.font1_family);
            widgets.font1_size_spin.set_value(config.frame.theme.font1_size);
            widgets.font2_btn.set_label(&config.frame.theme.font2_family);
            widgets.font2_size_spin.set_value(config.frame.theme.font2_size);
        }

        // Rebuild content tabs
        Self::rebuild_content_tabs(
            &self.config,
            &self.on_change,
            &self.preview,
            &self.content_notebook,
            &self.source_summaries,
            &self.available_fields,
            &self.theme_ref_refreshers,
        );

        self.preview.queue_draw();
    }

    pub fn set_source_summaries(&self, summaries: Vec<(String, String, usize, u32)>) {
        log::debug!(
            "CyberpunkConfigWidget::set_source_summaries - received {} summaries",
            summaries.len()
        );
        for (slot_name, _, group_num, item_idx) in &summaries {
            log::debug!("  summary: slot='{}', group={}, item={}", slot_name, group_num, item_idx);
        }
        // Extract group configuration from summaries
        let mut group_item_counts: std::collections::HashMap<usize, u32> = std::collections::HashMap::new();
        for (_, _, group_num, item_idx) in &summaries {
            let current_max = group_item_counts.entry(*group_num).or_insert(0);
            if *item_idx > *current_max {
                *current_max = *item_idx;
            }
        }

        // Convert to sorted vec
        let mut group_nums: Vec<usize> = group_item_counts.keys().cloned().collect();
        group_nums.sort();
        let group_counts: Vec<usize> = group_nums.iter()
            .map(|n| *group_item_counts.get(n).unwrap_or(&0) as usize)
            .collect();

        // Update the frame config with group information
        {
            let mut cfg = self.config.borrow_mut();
            let new_group_count = group_nums.len();
            cfg.frame.group_count = new_group_count;
            cfg.frame.group_item_counts = group_counts;

            // Ensure group_size_weights has the right length
            while cfg.frame.group_size_weights.len() < new_group_count {
                cfg.frame.group_size_weights.push(1.0);
            }
            // Trim if we have fewer groups now
            cfg.frame.group_size_weights.truncate(new_group_count);
        }

        *self.source_summaries.borrow_mut() = summaries;

        // Rebuild group weight spinners in Layout tab
        if let Some(widgets) = self.layout_widgets.borrow().as_ref() {
            Self::rebuild_group_spinners(
                &self.config,
                &self.on_change,
                &self.preview,
                &widgets.group_weights_box,
            );
        }

        Self::rebuild_content_tabs(
            &self.config,
            &self.on_change,
            &self.preview,
            &self.content_notebook,
            &self.source_summaries,
            &self.available_fields,
            &self.theme_ref_refreshers,
        );

        // Notify that config has changed so displayer gets updated
        if let Some(cb) = self.on_change.borrow().as_ref() {
            cb();
        }
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
