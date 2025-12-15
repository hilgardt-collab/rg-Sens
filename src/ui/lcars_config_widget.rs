//! LCARS Combo configuration widget
//!
//! Provides a tabbed interface for configuring all aspects of the LCARS display.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Entry, Label, Notebook, Orientation,
    Scale, SpinButton, StringList, ScrolledWindow,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::clipboard::CLIPBOARD;
use crate::ui::shared_font_dialog::shared_font_dialog;
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::lcars_display::{
    render_lcars_frame, render_content_background, SidebarPosition,
    ExtensionMode, CornerStyle, HeaderPosition, HeaderShape, HeaderAlign, SegmentConfig,
    DividerCapStyle, SplitOrientation, ContentDisplayType,
};
use crate::ui::background::Color;
use crate::ui::graph_config_widget::GraphConfigWidget;
use crate::ui::bar_config_widget::BarConfigWidget;
use crate::ui::core_bars_config_widget::CoreBarsConfigWidget;
use crate::displayers::LcarsDisplayConfig;
use crate::core::{FieldMetadata, FieldType, FieldPurpose};

/// Holds references to Frame tab widgets for updating when config changes
struct FrameWidgets {
    sidebar_spin: SpinButton,
    pos_dropdown: DropDown,
    ext_dropdown: DropDown,
    top_spin: SpinButton,
    bottom_spin: SpinButton,
    corner_spin: SpinButton,
    ext_corner_dropdown: DropDown,
    content_color_widget: Rc<ColorButtonWidget>,
    padding_spin: SpinButton,
}

/// Holds references to Headers tab widgets for updating when config changes
struct HeadersWidgets {
    // Top header
    top_show_check: CheckButton,
    top_text_entry: Entry,
    top_shape_dropdown: DropDown,
    top_bg_widget: Rc<ColorButtonWidget>,
    top_text_color_widget: Rc<ColorButtonWidget>,
    top_font_btn: Button,
    top_font_size_spin: SpinButton,
    top_bold_check: CheckButton,
    top_align_dropdown: DropDown,
    // Bottom header
    bottom_show_check: CheckButton,
    bottom_text_entry: Entry,
    bottom_shape_dropdown: DropDown,
    bottom_bg_widget: Rc<ColorButtonWidget>,
    bottom_text_color_widget: Rc<ColorButtonWidget>,
    bottom_font_btn: Button,
    bottom_font_size_spin: SpinButton,
    bottom_bold_check: CheckButton,
    bottom_align_dropdown: DropDown,
}

/// Holds references to Segments tab widgets for updating when config changes
struct SegmentsWidgets {
    count_spin: SpinButton,
    segment_frames: Rc<RefCell<Vec<gtk4::Frame>>>,
    // Store segment widget refs: (label_entry, color_widget, label_color_widget, weight_spin, font_btn, size_spin)
    segment_widgets: Rc<RefCell<Vec<(Entry, Rc<ColorButtonWidget>, Rc<ColorButtonWidget>, SpinButton, Button, SpinButton)>>>,
}

/// Holds references to Content tab widgets for updating when config changes
struct ContentWidgets {
    spacing_spin: SpinButton,
}

/// Holds references to Layout tab widgets for updating when config changes
struct SplitWidgets {
    orient_dropdown: DropDown,
    divider_spin: SpinButton,
    div_color_widget: Rc<ColorButtonWidget>,
    start_cap_dropdown: DropDown,
    end_cap_dropdown: DropDown,
    /// Container for group size weight spinners (rebuilt when groups change)
    group_weights_box: GtkBox,
    /// Checkbox for syncing segments with groups
    sync_segments_check: CheckButton,
}

/// Holds references to Animation tab widgets for updating when config changes
struct AnimationWidgets {
    enable_check: CheckButton,
    speed_scale: Scale,
}

/// LCARS configuration widget
pub struct LcarsConfigWidget {
    container: GtkBox,
    config: Rc<RefCell<LcarsDisplayConfig>>,
    on_change: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    preview: DrawingArea,
    /// Notebook for per-slot content configuration
    content_notebook: Rc<RefCell<Notebook>>,
    /// Source summaries for labeling tabs (slot_name, summary, group_num, item_idx)
    source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>>,
    /// Available fields from the source for text overlay configuration
    available_fields: Rc<RefCell<Vec<FieldMetadata>>>,
    /// Frame tab widgets for updating on set_config
    frame_widgets: Rc<RefCell<Option<FrameWidgets>>>,
    /// Headers tab widgets
    headers_widgets: Rc<RefCell<Option<HeadersWidgets>>>,
    /// Segments tab widgets
    segments_widgets: Rc<RefCell<Option<SegmentsWidgets>>>,
    /// Content tab widgets
    content_widgets: Rc<RefCell<Option<ContentWidgets>>>,
    /// Layout tab widgets
    split_widgets: Rc<RefCell<Option<SplitWidgets>>>,
    /// Animation tab widgets
    animation_widgets: Rc<RefCell<Option<AnimationWidgets>>>,
}

impl LcarsConfigWidget {
    pub fn new(available_fields: Vec<FieldMetadata>) -> Self {
        log::info!("=== LcarsConfigWidget::new() called with {} fields ===", available_fields.len());
        let container = GtkBox::new(Orientation::Vertical, 12);
        let config = Rc::new(RefCell::new(LcarsDisplayConfig::default()));
        let on_change: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let source_summaries: Rc<RefCell<Vec<(String, String, usize, u32)>>> = Rc::new(RefCell::new(Vec::new()));
        let available_fields: Rc<RefCell<Vec<FieldMetadata>>> = Rc::new(RefCell::new(available_fields));
        let frame_widgets: Rc<RefCell<Option<FrameWidgets>>> = Rc::new(RefCell::new(None));
        let headers_widgets: Rc<RefCell<Option<HeadersWidgets>>> = Rc::new(RefCell::new(None));
        let segments_widgets: Rc<RefCell<Option<SegmentsWidgets>>> = Rc::new(RefCell::new(None));
        let content_widgets: Rc<RefCell<Option<ContentWidgets>>> = Rc::new(RefCell::new(None));
        let split_widgets: Rc<RefCell<Option<SplitWidgets>>> = Rc::new(RefCell::new(None));
        let animation_widgets: Rc<RefCell<Option<AnimationWidgets>>> = Rc::new(RefCell::new(None));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(200);
        preview.set_vexpand(false);

        let config_clone = config.clone();
        preview.set_draw_func(move |_, cr, width, height| {
            // Dark background for preview
            cr.set_source_rgb(0.1, 0.1, 0.1);
            cr.paint().ok();

            let cfg = config_clone.borrow();
            let _ = render_lcars_frame(cr, &cfg.frame, width as f64, height as f64);
            let _ = render_content_background(cr, &cfg.frame, width as f64, height as f64);
        });

        // Create notebook for tabbed interface
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // Tab 1: Frame
        let frame_page = Self::create_frame_page(&config, &on_change, &preview, &frame_widgets);
        notebook.append_page(&frame_page, Some(&Label::new(Some("Frame"))));

        // Tab 2: Headers
        let headers_page = Self::create_headers_page(&config, &on_change, &preview, &headers_widgets);
        notebook.append_page(&headers_page, Some(&Label::new(Some("Headers"))));

        // Tab 3: Segments
        let segments_page = Self::create_segments_page(&config, &on_change, &preview, &segments_widgets, &split_widgets);
        notebook.append_page(&segments_page, Some(&Label::new(Some("Segments"))));

        // Tab 4: Content - with dynamic per-slot notebook
        let content_notebook = Rc::new(RefCell::new(Notebook::new()));
        let content_page = Self::create_content_page(&config, &on_change, &preview, &content_notebook, &source_summaries, &content_widgets, &available_fields);
        notebook.append_page(&content_page, Some(&Label::new(Some("Content"))));

        // Tab 5: Layout
        let split_page = Self::create_split_page(&config, &on_change, &preview, &split_widgets);
        notebook.append_page(&split_page, Some(&Label::new(Some("Layout"))));

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
            headers_widgets,
            segments_widgets,
            content_widgets,
            split_widgets,
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
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        frame_widgets_out: &Rc<RefCell<Option<FrameWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Sidebar width
        let sidebar_box = GtkBox::new(Orientation::Horizontal, 6);
        sidebar_box.append(&Label::new(Some("Sidebar Width:")));
        let sidebar_spin = SpinButton::with_range(50.0, 300.0, 5.0);
        sidebar_spin.set_value(config.borrow().frame.sidebar_width);
        sidebar_spin.set_hexpand(true);
        sidebar_box.append(&sidebar_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        sidebar_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.sidebar_width = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&sidebar_box);

        // Sidebar position
        let pos_box = GtkBox::new(Orientation::Horizontal, 6);
        pos_box.append(&Label::new(Some("Sidebar Position:")));
        let pos_list = StringList::new(&["Left", "Right"]);
        let pos_dropdown = DropDown::new(Some(pos_list), None::<gtk4::Expression>);
        let pos_idx = match config.borrow().frame.sidebar_position {
            SidebarPosition::Left => 0,
            SidebarPosition::Right => 1,
        };
        pos_dropdown.set_selected(pos_idx);
        pos_dropdown.set_hexpand(true);
        pos_box.append(&pos_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        pos_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.sidebar_position = match dropdown.selected() {
                0 => SidebarPosition::Left,
                _ => SidebarPosition::Right,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&pos_box);

        // Extension mode
        let ext_box = GtkBox::new(Orientation::Horizontal, 6);
        ext_box.append(&Label::new(Some("Extensions:")));
        let ext_list = StringList::new(&["Top", "Bottom", "Both", "None"]);
        let ext_dropdown = DropDown::new(Some(ext_list), None::<gtk4::Expression>);
        let ext_idx = match config.borrow().frame.extension_mode {
            ExtensionMode::Top => 0,
            ExtensionMode::Bottom => 1,
            ExtensionMode::Both => 2,
            ExtensionMode::None => 3,
        };
        ext_dropdown.set_selected(ext_idx);
        ext_dropdown.set_hexpand(true);
        ext_box.append(&ext_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        ext_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.extension_mode = match dropdown.selected() {
                0 => ExtensionMode::Top,
                1 => ExtensionMode::Bottom,
                2 => ExtensionMode::Both,
                _ => ExtensionMode::None,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&ext_box);

        // Top bar height
        let top_box = GtkBox::new(Orientation::Horizontal, 6);
        top_box.append(&Label::new(Some("Top Bar Height:")));
        let top_spin = SpinButton::with_range(20.0, 100.0, 2.0);
        top_spin.set_value(config.borrow().frame.top_bar_height);
        top_spin.set_hexpand(true);
        top_box.append(&top_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.top_bar_height = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&top_box);

        // Bottom bar height
        let bottom_box = GtkBox::new(Orientation::Horizontal, 6);
        bottom_box.append(&Label::new(Some("Bottom Bar Height:")));
        let bottom_spin = SpinButton::with_range(20.0, 100.0, 2.0);
        bottom_spin.set_value(config.borrow().frame.bottom_bar_height);
        bottom_spin.set_hexpand(true);
        bottom_box.append(&bottom_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.bottom_bar_height = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&bottom_box);

        // Corner radius
        let corner_box = GtkBox::new(Orientation::Horizontal, 6);
        corner_box.append(&Label::new(Some("Corner Radius:")));
        let corner_spin = SpinButton::with_range(0.0, 100.0, 5.0);
        corner_spin.set_value(config.borrow().frame.corner_radius);
        corner_spin.set_hexpand(true);
        corner_box.append(&corner_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        corner_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.corner_radius = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&corner_box);

        // Extension corner style
        let ext_corner_box = GtkBox::new(Orientation::Horizontal, 6);
        ext_corner_box.append(&Label::new(Some("Extension Corners:")));
        let ext_corner_list = StringList::new(&["Square", "Round"]);
        let ext_corner_dropdown = DropDown::new(Some(ext_corner_list), None::<gtk4::Expression>);
        let ext_corner_idx = match config.borrow().frame.extension_corner_style {
            CornerStyle::Square => 0,
            CornerStyle::Round => 1,
        };
        ext_corner_dropdown.set_selected(ext_corner_idx);
        ext_corner_dropdown.set_hexpand(true);
        ext_corner_box.append(&ext_corner_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        ext_corner_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.extension_corner_style = match dropdown.selected() {
                0 => CornerStyle::Square,
                _ => CornerStyle::Round,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&ext_corner_box);

        // Content background color
        let content_color_box = GtkBox::new(Orientation::Horizontal, 6);
        content_color_box.append(&Label::new(Some("Content Background:")));
        let content_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.content_bg_color));
        content_color_box.append(content_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        content_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.content_bg_color = color;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        page.append(&content_color_box);

        // Content padding
        let padding_box = GtkBox::new(Orientation::Horizontal, 6);
        padding_box.append(&Label::new(Some("Content Padding:")));
        let padding_spin = SpinButton::with_range(0.0, 50.0, 1.0);
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

        // Store widget references for updating when config changes
        *frame_widgets_out.borrow_mut() = Some(FrameWidgets {
            sidebar_spin: sidebar_spin.clone(),
            pos_dropdown: pos_dropdown.clone(),
            ext_dropdown: ext_dropdown.clone(),
            top_spin: top_spin.clone(),
            bottom_spin: bottom_spin.clone(),
            corner_spin: corner_spin.clone(),
            ext_corner_dropdown: ext_corner_dropdown.clone(),
            content_color_widget: content_color_widget.clone(),
            padding_spin: padding_spin.clone(),
        });

        page
    }

    fn create_headers_page(
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        headers_widgets_out: &Rc<RefCell<Option<HeadersWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Top Header section
        let top_label = Label::new(Some("Top Header"));
        top_label.set_halign(gtk4::Align::Start);
        top_label.add_css_class("heading");
        page.append(&top_label);

        // Top header show toggle (replaces position dropdown)
        let top_show_check = CheckButton::with_label("Show Top Header");
        top_show_check.set_active(config.borrow().frame.top_header.position == HeaderPosition::Top);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_show_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.top_header.position = if check.is_active() {
                HeaderPosition::Top
            } else {
                HeaderPosition::None
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&top_show_check);

        // Top header text
        let top_text_box = GtkBox::new(Orientation::Horizontal, 6);
        top_text_box.append(&Label::new(Some("Text:")));
        let top_text_entry = Entry::new();
        top_text_entry.set_text(&config.borrow().frame.top_header.text);
        top_text_entry.set_hexpand(true);
        top_text_box.append(&top_text_entry);

        // Copy/Paste text buttons
        let top_copy_text_btn = Button::with_label("Copy");
        let top_paste_text_btn = Button::with_label("Paste");
        top_text_box.append(&top_copy_text_btn);
        top_text_box.append(&top_paste_text_btn);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_text_entry.connect_changed(move |entry| {
            config_clone.borrow_mut().frame.top_header.text = entry.text().to_string();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Copy text handler
        let top_text_entry_clone = top_text_entry.clone();
        top_copy_text_btn.connect_clicked(move |_| {
            let text = top_text_entry_clone.text().to_string();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_text(text);
            }
        });

        // Paste text handler
        let top_text_entry_clone = top_text_entry.clone();
        top_paste_text_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(text) = clipboard.paste_text() {
                    top_text_entry_clone.set_text(&text);
                }
            }
        });
        page.append(&top_text_box);

        // Top header shape
        let top_shape_box = GtkBox::new(Orientation::Horizontal, 6);
        top_shape_box.append(&Label::new(Some("Shape:")));
        let top_shape_list = StringList::new(&["Pill", "Square"]);
        let top_shape_dropdown = DropDown::new(Some(top_shape_list), None::<gtk4::Expression>);
        let top_shape_idx = match config.borrow().frame.top_header.shape {
            HeaderShape::Pill => 0,
            HeaderShape::Square => 1,
        };
        top_shape_dropdown.set_selected(top_shape_idx);
        top_shape_dropdown.set_hexpand(true);
        top_shape_box.append(&top_shape_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_shape_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.top_header.shape = match dropdown.selected() {
                0 => HeaderShape::Pill,
                _ => HeaderShape::Square,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&top_shape_box);

        // Top header size percentages
        let top_size_box = GtkBox::new(Orientation::Horizontal, 6);
        top_size_box.append(&Label::new(Some("Height %:")));
        let top_height_spin = SpinButton::with_range(10.0, 100.0, 5.0);
        top_height_spin.set_value(config.borrow().frame.top_header.height_percent * 100.0);
        top_height_spin.set_width_chars(4);
        top_size_box.append(&top_height_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_height_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.top_header.height_percent = spin.value() / 100.0;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        top_size_box.append(&Label::new(Some("Width %:")));
        let top_width_spin = SpinButton::with_range(10.0, 100.0, 5.0);
        top_width_spin.set_value(config.borrow().frame.top_header.width_percent * 100.0);
        top_width_spin.set_width_chars(4);
        top_size_box.append(&top_width_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.top_header.width_percent = spin.value() / 100.0;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&top_size_box);

        // Top header colors row
        let top_colors_box = GtkBox::new(Orientation::Horizontal, 6);
        top_colors_box.append(&Label::new(Some("Background:")));
        let top_bg_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.top_header.bg_color));
        top_colors_box.append(top_bg_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_bg_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.top_header.bg_color = color;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        top_colors_box.append(&Label::new(Some("Text:")));
        let top_text_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.top_header.text_color));
        top_colors_box.append(top_text_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_text_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.top_header.text_color = color;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        page.append(&top_colors_box);

        // Top header font settings (with font dialog button)
        let top_font_box = GtkBox::new(Orientation::Horizontal, 6);
        top_font_box.append(&Label::new(Some("Font:")));

        let top_font_initial = {
            let cfg = config.borrow();
            let font = if cfg.frame.top_header.font.is_empty() {
                "Sans"
            } else {
                &cfg.frame.top_header.font
            };
            format!("{} {:.0}", font, cfg.frame.top_header.font_size)
        };
        let top_font_btn = Button::with_label(&top_font_initial);
        top_font_btn.set_hexpand(true);
        top_font_box.append(&top_font_btn);

        top_font_box.append(&Label::new(Some("Size:")));
        let top_font_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        top_font_size_spin.set_value(config.borrow().frame.top_header.font_size);
        top_font_size_spin.set_width_chars(4);
        top_font_box.append(&top_font_size_spin);

        // Copy/Paste font buttons
        let top_copy_font_btn = Button::with_label("Copy");
        let top_paste_font_btn = Button::with_label("Paste");
        top_font_box.append(&top_copy_font_btn);
        top_font_box.append(&top_paste_font_btn);

        // Bold checkbox
        let top_bold_check = CheckButton::with_label("Bold");
        top_bold_check.set_active(config.borrow().frame.top_header.font_bold);
        top_font_box.append(&top_bold_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_bold_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.top_header.font_bold = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Font button click handler
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let top_font_btn_clone = top_font_btn.clone();
        let top_font_size_spin_clone = top_font_size_spin.clone();
        top_font_btn.connect_clicked(move |button| {
            let window = button.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
            let config_ref = config_clone.clone();
            let on_change_ref = on_change_clone.clone();
            let preview_ref = preview_clone.clone();
            let btn_ref = top_font_btn_clone.clone();
            let spin_ref = top_font_size_spin_clone.clone();

            let font_dialog = shared_font_dialog();
            let current_font = {
                let cfg = config_ref.borrow();
                format!("{} {}", cfg.frame.top_header.font, cfg.frame.top_header.font_size as i32)
            };
            let pango_desc = gtk4::pango::FontDescription::from_string(&current_font);

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Ok(desc) = font_dialog.choose_font_future(window.as_ref(), Some(&pango_desc)).await {
                    let family = desc.family().map(|f| f.to_string()).unwrap_or_else(|| "Sans".to_string());
                    let size = (desc.size() / gtk4::pango::SCALE) as f64;

                    {
                        let mut cfg = config_ref.borrow_mut();
                        cfg.frame.top_header.font = family.clone();
                        cfg.frame.top_header.font_size = size;
                    }

                    btn_ref.set_label(&format!("{} {:.0}", family, size));
                    spin_ref.set_value(size);
                    preview_ref.queue_draw();
                    if let Some(cb) = on_change_ref.borrow().as_ref() {
                        cb();
                    }
                }
            });
        });

        // Font size spin handler
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let top_font_btn_clone = top_font_btn.clone();
        top_font_size_spin.connect_value_changed(move |spin| {
            let new_size = spin.value();
            let family = {
                let f = config_clone.borrow().frame.top_header.font.clone();
                if f.is_empty() { "Sans".to_string() } else { f }
            };
            config_clone.borrow_mut().frame.top_header.font_size = new_size;
            top_font_btn_clone.set_label(&format!("{} {:.0}", family, new_size));
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Copy font handler
        let config_clone = config.clone();
        top_copy_font_btn.connect_clicked(move |_| {
            let cfg = config_clone.borrow();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_font(cfg.frame.top_header.font.clone(), cfg.frame.top_header.font_size, false, false);
            }
        });

        // Paste font handler
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let top_font_btn_clone = top_font_btn.clone();
        let top_font_size_spin_clone = top_font_size_spin.clone();
        top_paste_font_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some((family, size, _bold, _italic)) = clipboard.paste_font() {
                    {
                        let mut cfg = config_clone.borrow_mut();
                        cfg.frame.top_header.font = family.clone();
                        cfg.frame.top_header.font_size = size;
                    }
                    top_font_btn_clone.set_label(&format!("{} {:.0}", family, size));
                    top_font_size_spin_clone.set_value(size);
                    Self::queue_redraw(&preview_clone, &on_change_clone);
                }
            }
        });
        page.append(&top_font_box);

        // Top header alignment (relative to side extension)
        let top_align_box = GtkBox::new(Orientation::Horizontal, 6);
        top_align_box.append(&Label::new(Some("Align (from sidebar):")));
        let top_align_list = StringList::new(&["Near", "Center", "Far"]);
        let top_align_dropdown = DropDown::new(Some(top_align_list), None::<gtk4::Expression>);
        let top_align_idx = match config.borrow().frame.top_header.align {
            HeaderAlign::Left => 0,
            HeaderAlign::Center => 1,
            HeaderAlign::Right => 2,
        };
        top_align_dropdown.set_selected(top_align_idx);
        top_align_dropdown.set_hexpand(true);
        top_align_box.append(&top_align_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_align_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.top_header.align = match dropdown.selected() {
                0 => HeaderAlign::Left,
                1 => HeaderAlign::Center,
                _ => HeaderAlign::Right,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&top_align_box);

        // Separator
        page.append(&gtk4::Separator::new(Orientation::Horizontal));

        // Bottom Header section
        let bottom_label = Label::new(Some("Bottom Header"));
        bottom_label.set_halign(gtk4::Align::Start);
        bottom_label.add_css_class("heading");
        page.append(&bottom_label);

        // Bottom header show toggle (replaces position dropdown)
        let bottom_show_check = CheckButton::with_label("Show Bottom Header");
        bottom_show_check.set_active(config.borrow().frame.bottom_header.position == HeaderPosition::Bottom);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_show_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.bottom_header.position = if check.is_active() {
                HeaderPosition::Bottom
            } else {
                HeaderPosition::None
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&bottom_show_check);

        // Bottom header text
        let bottom_text_box = GtkBox::new(Orientation::Horizontal, 6);
        bottom_text_box.append(&Label::new(Some("Text:")));
        let bottom_text_entry = Entry::new();
        bottom_text_entry.set_text(&config.borrow().frame.bottom_header.text);
        bottom_text_entry.set_hexpand(true);
        bottom_text_box.append(&bottom_text_entry);

        // Copy/Paste text buttons
        let bottom_copy_text_btn = Button::with_label("Copy");
        let bottom_paste_text_btn = Button::with_label("Paste");
        bottom_text_box.append(&bottom_copy_text_btn);
        bottom_text_box.append(&bottom_paste_text_btn);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_text_entry.connect_changed(move |entry| {
            config_clone.borrow_mut().frame.bottom_header.text = entry.text().to_string();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Copy text handler
        let bottom_text_entry_clone = bottom_text_entry.clone();
        bottom_copy_text_btn.connect_clicked(move |_| {
            let text = bottom_text_entry_clone.text().to_string();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_text(text);
            }
        });

        // Paste text handler
        let bottom_text_entry_clone = bottom_text_entry.clone();
        bottom_paste_text_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(text) = clipboard.paste_text() {
                    bottom_text_entry_clone.set_text(&text);
                }
            }
        });
        page.append(&bottom_text_box);

        // Bottom header shape
        let bottom_shape_box = GtkBox::new(Orientation::Horizontal, 6);
        bottom_shape_box.append(&Label::new(Some("Shape:")));
        let bottom_shape_list = StringList::new(&["Pill", "Square"]);
        let bottom_shape_dropdown = DropDown::new(Some(bottom_shape_list), None::<gtk4::Expression>);
        let bottom_shape_idx = match config.borrow().frame.bottom_header.shape {
            HeaderShape::Pill => 0,
            HeaderShape::Square => 1,
        };
        bottom_shape_dropdown.set_selected(bottom_shape_idx);
        bottom_shape_dropdown.set_hexpand(true);
        bottom_shape_box.append(&bottom_shape_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_shape_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.bottom_header.shape = match dropdown.selected() {
                0 => HeaderShape::Pill,
                _ => HeaderShape::Square,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&bottom_shape_box);

        // Bottom header size percentages
        let bottom_size_box = GtkBox::new(Orientation::Horizontal, 6);
        bottom_size_box.append(&Label::new(Some("Height %:")));
        let bottom_height_spin = SpinButton::with_range(10.0, 100.0, 5.0);
        bottom_height_spin.set_value(config.borrow().frame.bottom_header.height_percent * 100.0);
        bottom_height_spin.set_width_chars(4);
        bottom_size_box.append(&bottom_height_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_height_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.bottom_header.height_percent = spin.value() / 100.0;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        bottom_size_box.append(&Label::new(Some("Width %:")));
        let bottom_width_spin = SpinButton::with_range(10.0, 100.0, 5.0);
        bottom_width_spin.set_value(config.borrow().frame.bottom_header.width_percent * 100.0);
        bottom_width_spin.set_width_chars(4);
        bottom_size_box.append(&bottom_width_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_width_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.bottom_header.width_percent = spin.value() / 100.0;
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&bottom_size_box);

        // Bottom header colors row
        let bottom_colors_box = GtkBox::new(Orientation::Horizontal, 6);
        bottom_colors_box.append(&Label::new(Some("Background:")));
        let bottom_bg_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.bottom_header.bg_color));
        bottom_colors_box.append(bottom_bg_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_bg_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.bottom_header.bg_color = color;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        bottom_colors_box.append(&Label::new(Some("Text:")));
        let bottom_text_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.bottom_header.text_color));
        bottom_colors_box.append(bottom_text_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_text_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.bottom_header.text_color = color;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        page.append(&bottom_colors_box);

        // Bottom header font settings (with font dialog button)
        let bottom_font_box = GtkBox::new(Orientation::Horizontal, 6);
        bottom_font_box.append(&Label::new(Some("Font:")));

        let bottom_font_initial = {
            let cfg = config.borrow();
            let font = if cfg.frame.bottom_header.font.is_empty() {
                "Sans"
            } else {
                &cfg.frame.bottom_header.font
            };
            format!("{} {:.0}", font, cfg.frame.bottom_header.font_size)
        };
        let bottom_font_btn = Button::with_label(&bottom_font_initial);
        bottom_font_btn.set_hexpand(true);
        bottom_font_box.append(&bottom_font_btn);

        bottom_font_box.append(&Label::new(Some("Size:")));
        let bottom_font_size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
        bottom_font_size_spin.set_value(config.borrow().frame.bottom_header.font_size);
        bottom_font_size_spin.set_width_chars(4);
        bottom_font_box.append(&bottom_font_size_spin);

        // Copy/Paste font buttons
        let bottom_copy_font_btn = Button::with_label("Copy");
        let bottom_paste_font_btn = Button::with_label("Paste");
        bottom_font_box.append(&bottom_copy_font_btn);
        bottom_font_box.append(&bottom_paste_font_btn);

        // Bold checkbox
        let bottom_bold_check = CheckButton::with_label("Bold");
        bottom_bold_check.set_active(config.borrow().frame.bottom_header.font_bold);
        bottom_font_box.append(&bottom_bold_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_bold_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.bottom_header.font_bold = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Font button click handler
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let bottom_font_btn_clone = bottom_font_btn.clone();
        let bottom_font_size_spin_clone = bottom_font_size_spin.clone();
        bottom_font_btn.connect_clicked(move |button| {
            let window = button.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
            let config_ref = config_clone.clone();
            let on_change_ref = on_change_clone.clone();
            let preview_ref = preview_clone.clone();
            let btn_ref = bottom_font_btn_clone.clone();
            let spin_ref = bottom_font_size_spin_clone.clone();

            let font_dialog = shared_font_dialog();
            let current_font = {
                let cfg = config_ref.borrow();
                format!("{} {}", cfg.frame.bottom_header.font, cfg.frame.bottom_header.font_size as i32)
            };
            let pango_desc = gtk4::pango::FontDescription::from_string(&current_font);

            gtk4::glib::MainContext::default().spawn_local(async move {
                if let Ok(desc) = font_dialog.choose_font_future(window.as_ref(), Some(&pango_desc)).await {
                    let family = desc.family().map(|f| f.to_string()).unwrap_or_else(|| "Sans".to_string());
                    let size = (desc.size() / gtk4::pango::SCALE) as f64;

                    {
                        let mut cfg = config_ref.borrow_mut();
                        cfg.frame.bottom_header.font = family.clone();
                        cfg.frame.bottom_header.font_size = size;
                    }

                    btn_ref.set_label(&format!("{} {:.0}", family, size));
                    spin_ref.set_value(size);
                    preview_ref.queue_draw();
                    if let Some(cb) = on_change_ref.borrow().as_ref() {
                        cb();
                    }
                }
            });
        });

        // Font size spin handler
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let bottom_font_btn_clone = bottom_font_btn.clone();
        bottom_font_size_spin.connect_value_changed(move |spin| {
            let new_size = spin.value();
            let family = {
                let f = config_clone.borrow().frame.bottom_header.font.clone();
                if f.is_empty() { "Sans".to_string() } else { f }
            };
            config_clone.borrow_mut().frame.bottom_header.font_size = new_size;
            bottom_font_btn_clone.set_label(&format!("{} {:.0}", family, new_size));
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Copy font handler
        let config_clone = config.clone();
        bottom_copy_font_btn.connect_clicked(move |_| {
            let cfg = config_clone.borrow();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_font(cfg.frame.bottom_header.font.clone(), cfg.frame.bottom_header.font_size, false, false);
            }
        });

        // Paste font handler
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let bottom_font_btn_clone = bottom_font_btn.clone();
        let bottom_font_size_spin_clone = bottom_font_size_spin.clone();
        bottom_paste_font_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some((family, size, _bold, _italic)) = clipboard.paste_font() {
                    {
                        let mut cfg = config_clone.borrow_mut();
                        cfg.frame.bottom_header.font = family.clone();
                        cfg.frame.bottom_header.font_size = size;
                    }
                    bottom_font_btn_clone.set_label(&format!("{} {:.0}", family, size));
                    bottom_font_size_spin_clone.set_value(size);
                    Self::queue_redraw(&preview_clone, &on_change_clone);
                }
            }
        });
        page.append(&bottom_font_box);

        // Bottom header alignment (relative to side extension)
        let bottom_align_box = GtkBox::new(Orientation::Horizontal, 6);
        bottom_align_box.append(&Label::new(Some("Align (from sidebar):")));
        let bottom_align_list = StringList::new(&["Near", "Center", "Far"]);
        let bottom_align_dropdown = DropDown::new(Some(bottom_align_list), None::<gtk4::Expression>);
        let bottom_align_idx = match config.borrow().frame.bottom_header.align {
            HeaderAlign::Left => 0,
            HeaderAlign::Center => 1,
            HeaderAlign::Right => 2,
        };
        bottom_align_dropdown.set_selected(bottom_align_idx);
        bottom_align_dropdown.set_hexpand(true);
        bottom_align_box.append(&bottom_align_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_align_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.bottom_header.align = match dropdown.selected() {
                0 => HeaderAlign::Left,
                1 => HeaderAlign::Center,
                _ => HeaderAlign::Right,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&bottom_align_box);

        // Store widget references for updating when config changes
        *headers_widgets_out.borrow_mut() = Some(HeadersWidgets {
            top_show_check: top_show_check.clone(),
            top_text_entry: top_text_entry.clone(),
            top_shape_dropdown: top_shape_dropdown.clone(),
            top_bg_widget: top_bg_widget.clone(),
            top_text_color_widget: top_text_color_widget.clone(),
            top_font_btn: top_font_btn.clone(),
            top_font_size_spin: top_font_size_spin.clone(),
            top_bold_check: top_bold_check.clone(),
            top_align_dropdown: top_align_dropdown.clone(),
            bottom_show_check: bottom_show_check.clone(),
            bottom_text_entry: bottom_text_entry.clone(),
            bottom_shape_dropdown: bottom_shape_dropdown.clone(),
            bottom_bg_widget: bottom_bg_widget.clone(),
            bottom_text_color_widget: bottom_text_color_widget.clone(),
            bottom_font_btn: bottom_font_btn.clone(),
            bottom_font_size_spin: bottom_font_size_spin.clone(),
            bottom_bold_check: bottom_bold_check.clone(),
            bottom_align_dropdown: bottom_align_dropdown.clone(),
        });

        page
    }

    fn create_segments_page(
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        segments_widgets_out: &Rc<RefCell<Option<SegmentsWidgets>>>,
        split_widgets: &Rc<RefCell<Option<SplitWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Segment count
        let count_box = GtkBox::new(Orientation::Horizontal, 6);
        count_box.append(&Label::new(Some("Number of Segments:")));
        let count_spin = SpinButton::with_range(0.0, 10.0, 1.0);
        count_spin.set_value(config.borrow().frame.segment_count as f64);
        count_spin.set_hexpand(true);
        count_box.append(&count_spin);
        page.append(&count_box);

        // Scrolled area for segment configs
        let scrolled = ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_min_content_height(200);

        let segments_box = GtkBox::new(Orientation::Vertical, 8);
        let segments_box_rc = Rc::new(segments_box);

        // Create wrapper container to hold segment frames
        let segment_frames: Rc<RefCell<Vec<gtk4::Frame>>> = Rc::new(RefCell::new(Vec::new()));

        // Store per-segment widget refs: (label_entry, color_widget, label_color_widget, weight_spin, font_btn, size_spin)
        let segment_widgets: Rc<RefCell<Vec<(Entry, Rc<ColorButtonWidget>, Rc<ColorButtonWidget>, SpinButton, Button, SpinButton)>>> = Rc::new(RefCell::new(Vec::new()));

        // Helper function to create a segment config widget
        // Returns (frame, (label_entry, color_widget, label_color_widget, weight_spin, font_btn, size_spin))
        let create_segment_widget = {
            let config = config.clone();
            let on_change = on_change.clone();
            let preview = preview.clone();
            move |seg_idx: usize| -> (gtk4::Frame, (Entry, Rc<ColorButtonWidget>, Rc<ColorButtonWidget>, SpinButton, Button, SpinButton)) {
                let seg_frame = gtk4::Frame::new(Some(&format!("Segment {}", seg_idx + 1)));
                let seg_box = GtkBox::new(Orientation::Vertical, 4);
                seg_box.set_margin_start(8);
                seg_box.set_margin_end(8);
                seg_box.set_margin_top(8);
                seg_box.set_margin_bottom(8);

                // Label
                let label_box = GtkBox::new(Orientation::Horizontal, 6);
                label_box.append(&Label::new(Some("Label:")));
                let label_entry = Entry::new();
                if let Some(seg) = config.borrow().frame.segments.get(seg_idx) {
                    label_entry.set_text(&seg.label);
                }
                label_entry.set_hexpand(true);
                label_box.append(&label_entry);

                let config_clone = config.clone();
                let on_change_clone = on_change.clone();
                let preview_clone = preview.clone();
                label_entry.connect_changed(move |entry| {
                    let mut cfg = config_clone.borrow_mut();
                    while cfg.frame.segments.len() <= seg_idx {
                        cfg.frame.segments.push(SegmentConfig::default());
                    }
                    cfg.frame.segments[seg_idx].label = entry.text().to_string();
                    drop(cfg);
                    Self::queue_redraw(&preview_clone, &on_change_clone);
                });
                seg_box.append(&label_box);

                // Colors row (segment color + label color)
                let colors_box = GtkBox::new(Orientation::Horizontal, 12);
                colors_box.append(&Label::new(Some("Segment:")));
                let seg_color = config.borrow().frame.segments.get(seg_idx)
                    .map(|s| s.color)
                    .unwrap_or_else(|| Color::new(0.8, 0.4, 0.4, 1.0));
                let color_widget = Rc::new(ColorButtonWidget::new(seg_color));
                colors_box.append(color_widget.widget());

                let config_clone = config.clone();
                let on_change_clone = on_change.clone();
                let preview_clone = preview.clone();
                color_widget.set_on_change(move |color| {
                    let mut cfg = config_clone.borrow_mut();
                    while cfg.frame.segments.len() <= seg_idx {
                        cfg.frame.segments.push(SegmentConfig::default());
                    }
                    cfg.frame.segments[seg_idx].color = color;
                    drop(cfg);
                    preview_clone.queue_draw();
                    if let Some(cb) = on_change_clone.borrow().as_ref() {
                        cb();
                    }
                });

                colors_box.append(&Label::new(Some("Label:")));
                let label_color = config.borrow().frame.segments.get(seg_idx)
                    .map(|s| s.label_color)
                    .unwrap_or_else(|| Color::new(0.0, 0.0, 0.0, 1.0));
                let label_color_widget = Rc::new(ColorButtonWidget::new(label_color));
                colors_box.append(label_color_widget.widget());

                let config_clone = config.clone();
                let on_change_clone = on_change.clone();
                let preview_clone = preview.clone();
                label_color_widget.set_on_change(move |color| {
                    let mut cfg = config_clone.borrow_mut();
                    while cfg.frame.segments.len() <= seg_idx {
                        cfg.frame.segments.push(SegmentConfig::default());
                    }
                    cfg.frame.segments[seg_idx].label_color = color;
                    drop(cfg);
                    preview_clone.queue_draw();
                    if let Some(cb) = on_change_clone.borrow().as_ref() {
                        cb();
                    }
                });
                seg_box.append(&colors_box);

                // Weight
                let weight_box = GtkBox::new(Orientation::Horizontal, 6);
                weight_box.append(&Label::new(Some("Height Weight:")));
                let weight_spin = SpinButton::with_range(0.1, 5.0, 0.1);
                if let Some(seg) = config.borrow().frame.segments.get(seg_idx) {
                    weight_spin.set_value(seg.height_weight);
                } else {
                    weight_spin.set_value(1.0);
                }
                weight_spin.set_hexpand(true);
                weight_box.append(&weight_spin);

                let config_clone = config.clone();
                let on_change_clone = on_change.clone();
                let preview_clone = preview.clone();
                weight_spin.connect_value_changed(move |spin| {
                    let mut cfg = config_clone.borrow_mut();
                    while cfg.frame.segments.len() <= seg_idx {
                        cfg.frame.segments.push(SegmentConfig::default());
                    }
                    cfg.frame.segments[seg_idx].height_weight = spin.value();
                    drop(cfg);
                    Self::queue_redraw(&preview_clone, &on_change_clone);
                });
                seg_box.append(&weight_box);

                // Font settings (with font dialog button + size spinner + copy/paste)
                let font_box = GtkBox::new(Orientation::Horizontal, 6);
                font_box.append(&Label::new(Some("Font:")));

                let (font_family, font_size) = {
                    let cfg = config.borrow();
                    if let Some(seg) = cfg.frame.segments.get(seg_idx) {
                        (seg.font.clone(), seg.font_size)
                    } else {
                        ("Sans".to_string(), 12.0)
                    }
                };

                let font_btn = Button::with_label(&format!("{} {:.0}", font_family, font_size));
                font_btn.set_hexpand(true);
                font_box.append(&font_btn);

                font_box.append(&Label::new(Some("Size:")));
                let size_spin = SpinButton::with_range(6.0, 72.0, 1.0);
                size_spin.set_value(font_size);
                size_spin.set_width_chars(4);
                font_box.append(&size_spin);

                // Copy/Paste font buttons
                let copy_font_btn = Button::with_label("Copy");
                let paste_font_btn = Button::with_label("Paste");
                font_box.append(&copy_font_btn);
                font_box.append(&paste_font_btn);

                // Font button click handler
                let config_clone = config.clone();
                let on_change_clone = on_change.clone();
                let preview_clone = preview.clone();
                let font_btn_clone = font_btn.clone();
                let size_spin_clone = size_spin.clone();
                font_btn.connect_clicked(move |button| {
                    let window = button.root().and_then(|root| root.downcast::<gtk4::Window>().ok());
                    let config_ref = config_clone.clone();
                    let on_change_ref = on_change_clone.clone();
                    let preview_ref = preview_clone.clone();
                    let btn_ref = font_btn_clone.clone();
                    let spin_ref = size_spin_clone.clone();

                    let font_dialog = shared_font_dialog();
                    let current_font = {
                        let cfg = config_ref.borrow();
                        if let Some(seg) = cfg.frame.segments.get(seg_idx) {
                            format!("{} {}", seg.font, seg.font_size as i32)
                        } else {
                            "Sans 12".to_string()
                        }
                    };
                    let pango_desc = gtk4::pango::FontDescription::from_string(&current_font);

                    gtk4::glib::MainContext::default().spawn_local(async move {
                        if let Ok(desc) = font_dialog.choose_font_future(window.as_ref(), Some(&pango_desc)).await {
                            let family = desc.family().map(|f| f.to_string()).unwrap_or_else(|| "Sans".to_string());
                            let size = (desc.size() / gtk4::pango::SCALE) as f64;

                            {
                                let mut cfg = config_ref.borrow_mut();
                                while cfg.frame.segments.len() <= seg_idx {
                                    cfg.frame.segments.push(SegmentConfig::default());
                                }
                                cfg.frame.segments[seg_idx].font = family.clone();
                                cfg.frame.segments[seg_idx].font_size = size;
                            }

                            btn_ref.set_label(&format!("{} {:.0}", family, size));
                            spin_ref.set_value(size);
                            preview_ref.queue_draw();
                            if let Some(cb) = on_change_ref.borrow().as_ref() {
                                cb();
                            }
                        }
                    });
                });

                // Size spin handler
                let config_clone = config.clone();
                let on_change_clone = on_change.clone();
                let preview_clone = preview.clone();
                let font_btn_clone = font_btn.clone();
                size_spin.connect_value_changed(move |spin| {
                    let new_size = spin.value();
                    let family = {
                        let cfg = config_clone.borrow();
                        cfg.frame.segments.get(seg_idx)
                            .map(|s| s.font.clone())
                            .unwrap_or_else(|| "Sans".to_string())
                    };
                    {
                        let mut cfg = config_clone.borrow_mut();
                        while cfg.frame.segments.len() <= seg_idx {
                            cfg.frame.segments.push(SegmentConfig::default());
                        }
                        cfg.frame.segments[seg_idx].font_size = new_size;
                    }
                    font_btn_clone.set_label(&format!("{} {:.0}", family, new_size));
                    Self::queue_redraw(&preview_clone, &on_change_clone);
                });

                // Copy font handler
                let config_clone = config.clone();
                copy_font_btn.connect_clicked(move |_| {
                    let cfg = config_clone.borrow();
                    if let Some(seg) = cfg.frame.segments.get(seg_idx) {
                        if let Ok(mut clipboard) = CLIPBOARD.lock() {
                            clipboard.copy_font(seg.font.clone(), seg.font_size, false, false);
                        }
                    }
                });

                // Paste font handler
                let config_clone = config.clone();
                let on_change_clone = on_change.clone();
                let preview_clone = preview.clone();
                let font_btn_clone = font_btn.clone();
                let size_spin_clone = size_spin.clone();
                paste_font_btn.connect_clicked(move |_| {
                    if let Ok(clipboard) = CLIPBOARD.lock() {
                        if let Some((family, size, _bold, _italic)) = clipboard.paste_font() {
                            {
                                let mut cfg = config_clone.borrow_mut();
                                while cfg.frame.segments.len() <= seg_idx {
                                    cfg.frame.segments.push(SegmentConfig::default());
                                }
                                cfg.frame.segments[seg_idx].font = family.clone();
                                cfg.frame.segments[seg_idx].font_size = size;
                            }
                            font_btn_clone.set_label(&format!("{} {:.0}", family, size));
                            size_spin_clone.set_value(size);
                            Self::queue_redraw(&preview_clone, &on_change_clone);
                        }
                    }
                });
                seg_box.append(&font_box);

                seg_frame.set_child(Some(&seg_box));
                (seg_frame, (label_entry, color_widget.clone(), label_color_widget.clone(), weight_spin.clone(), font_btn.clone(), size_spin.clone()))
            }
        };

        // Create initial segment widgets based on current count
        let initial_count = config.borrow().frame.segment_count as usize;
        for i in 0..10 {
            let (frame, widgets) = create_segment_widget(i);
            frame.set_visible(i < initial_count);
            segments_box_rc.append(&frame);
            segment_frames.borrow_mut().push(frame);
            segment_widgets.borrow_mut().push(widgets);
        }

        // Connect count spin to show/hide segment frames
        let segment_frames_clone = segment_frames.clone();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let split_widgets_clone = split_widgets.clone();
        count_spin.connect_value_changed(move |spin| {
            let count = spin.value() as usize;

            // Show/hide frames based on count
            let frames = segment_frames_clone.borrow();
            for (i, frame) in frames.iter().enumerate() {
                frame.set_visible(i < count);
            }

            // Update config
            let mut cfg = config_clone.borrow_mut();
            cfg.frame.segment_count = count as u32;

            // Ensure we have enough segments in config
            while cfg.frame.segments.len() < count {
                cfg.frame.segments.push(SegmentConfig::default());
            }

            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);

            // Update sync checkbox sensitivity
            if let Some(ref widgets) = *split_widgets_clone.borrow() {
                Self::update_sync_checkbox_sensitivity(&widgets.sync_segments_check, &config_clone);
            }
        });

        scrolled.set_child(Some(&*segments_box_rc));
        page.append(&scrolled);

        // Store widget references for updating when config changes
        *segments_widgets_out.borrow_mut() = Some(SegmentsWidgets {
            count_spin: count_spin.clone(),
            segment_frames: segment_frames.clone(),
            segment_widgets: segment_widgets.clone(),
        });

        page
    }

    fn create_content_page(
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        content_widgets_out: &Rc<RefCell<Option<ContentWidgets>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Item spacing
        let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
        spacing_box.append(&Label::new(Some("Item Spacing:")));
        let spacing_spin = SpinButton::with_range(0.0, 20.0, 1.0);
        spacing_spin.set_value(config.borrow().frame.item_spacing);
        spacing_spin.set_hexpand(true);
        spacing_box.append(&spacing_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        spacing_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.item_spacing = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&spacing_box);

        // Per-slot display configuration heading
        let slots_heading = Label::new(Some("Per-Slot Display Configuration"));
        slots_heading.add_css_class("heading");
        slots_heading.set_margin_top(12);
        slots_heading.set_halign(gtk4::Align::Start);
        page.append(&slots_heading);

        // Note about configuring sources first
        let note_label = Label::new(Some(
            "Configure data sources in the 'Data Source' tab first.\n\
             Each slot's tab shows its source and allows display type configuration."
        ));
        note_label.set_halign(gtk4::Align::Start);
        note_label.add_css_class("dim-label");
        page.append(&note_label);

        // Content notebook for per-slot configuration
        let nb = content_notebook.borrow();
        nb.set_scrollable(true);
        nb.set_vexpand(true);
        nb.set_margin_top(8);
        page.append(&*nb);
        drop(nb);

        // Build initial tabs based on source summaries
        Self::rebuild_content_notebook_tabs(content_notebook, source_summaries, config, on_change, preview, available_fields);

        // Store widget references for updating when config changes
        *content_widgets_out.borrow_mut() = Some(ContentWidgets {
            spacing_spin: spacing_spin.clone(),
        });

        page
    }

    /// Rebuild the content notebook tabs based on source summaries (organized by groups)
    fn rebuild_content_notebook_tabs(
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
    ) {
        let notebook = content_notebook.borrow();

        // Clear existing tabs
        while notebook.n_pages() > 0 {
            notebook.remove_page(Some(0));
        }

        let summaries = source_summaries.borrow();

        log::info!(
            "=== rebuild_content_notebook_tabs: source_summaries has {} entries ===",
            summaries.len()
        );

        if summaries.is_empty() {
            // Show placeholder when no sources configured
            log::warn!("rebuild_content_notebook_tabs: summaries is EMPTY, showing placeholder (need Combination source)");
            let placeholder = GtkBox::new(Orientation::Vertical, 8);
            placeholder.set_margin_start(12);
            placeholder.set_margin_end(12);
            placeholder.set_margin_top(12);
            let label = Label::new(Some("No sources configured.\nGo to 'Data Source' tab and select 'Combination' source to configure LCARS content."));
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

        // Create a tab for each group
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
                    let tab_label = format!("Item {} - {}", item_idx, summary);
                    let tab_box = Self::create_slot_config_tab(&slot_name, config, on_change, preview, available_fields);
                    items_notebook.append_page(&tab_box, Some(&Label::new(Some(&tab_label))));
                }

                group_box.append(&items_notebook);
                notebook.append_page(&group_box, Some(&Label::new(Some(&format!("Group {}", group_num)))));
            }
        }
    }

    /// Create configuration tab for a single slot
    fn create_slot_config_tab(
        slot_name: &str,
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
    ) -> GtkBox {
        log::info!("=== create_slot_config_tab() called for slot '{}' ===", slot_name);
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

        // Display type dropdown (removed Level Bar - functionality in Bar)
        let type_box = GtkBox::new(Orientation::Horizontal, 6);
        type_box.append(&Label::new(Some("Display As:")));
        let type_list = StringList::new(&["Bar", "Text", "Graph", "Core Bars"]);
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
            ContentDisplayType::Bar | ContentDisplayType::LevelBar => 0, // LevelBar falls back to Bar
            ContentDisplayType::Text => 1,
            ContentDisplayType::Graph => 2,
            ContentDisplayType::CoreBars => 3,
        };
        type_dropdown.set_selected(type_idx);
        type_box.append(&type_dropdown);
        inner_box.append(&type_box);

        // Item height
        let height_box = GtkBox::new(Orientation::Horizontal, 6);
        height_box.append(&Label::new(Some("Item Height:")));
        let height_spin = SpinButton::with_range(20.0, 200.0, 5.0);
        let current_height = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.item_height)
                .unwrap_or(60.0)
        };
        height_spin.set_value(current_height);
        height_spin.set_hexpand(true);
        height_box.append(&height_spin);
        inner_box.append(&height_box);

        // === Bar Configuration Section (Modular Widget) ===
        // Uses the reusable BarConfigWidget which includes bar style, colors, and text overlay settings
        let bar_config_frame = gtk4::Frame::new(Some("Bar Configuration"));
        bar_config_frame.set_margin_top(12);

        // Get available fields from the source
        // Filter to fields relevant to this slot (prefixed with slot_name_)
        let slot_prefix = format!("{}_", slot_name);
        let source_fields = available_fields.borrow();
        let mut lcars_fields: Vec<FieldMetadata> = source_fields.iter()
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
        if lcars_fields.is_empty() {
            lcars_fields = vec![
                FieldMetadata::new("caption", "Caption", "Label text for the item", FieldType::Text, FieldPurpose::Caption),
                FieldMetadata::new("value", "Value", "Current value with formatting", FieldType::Text, FieldPurpose::Value),
                FieldMetadata::new("unit", "Unit", "Unit of measurement", FieldType::Text, FieldPurpose::Unit),
                FieldMetadata::new("numerical_value", "Numeric Value", "Raw numeric value", FieldType::Numerical, FieldPurpose::Value),
                FieldMetadata::new("min_value", "Minimum", "Minimum value for range", FieldType::Numerical, FieldPurpose::Other),
                FieldMetadata::new("max_value", "Maximum", "Maximum value for range", FieldType::Numerical, FieldPurpose::Other),
            ];
        }
        drop(source_fields);

        // Create BarConfigWidget for bar configuration
        let bar_widget = BarConfigWidget::new(lcars_fields.clone());

        // Initialize with current config if exists
        let current_bar_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.bar_config.clone())
                .unwrap_or_default()
        };
        bar_widget.set_config(current_bar_config);

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

        bar_config_frame.set_child(Some(bar_widget_rc.widget()));
        inner_box.append(&bar_config_frame);

        // === Graph Configuration Section ===
        // Embed the actual GraphConfigWidget for full configuration
        let graph_config_frame = gtk4::Frame::new(Some("Graph Configuration"));
        graph_config_frame.set_margin_top(12);

        // Create GraphConfigWidget with LCARS fields for text overlay configuration
        let graph_widget = GraphConfigWidget::new(lcars_fields.clone());

        // Initialize with current config if exists
        let current_graph_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.graph_config.clone())
                .unwrap_or_default()
        };
        log::info!(
            "=== LcarsConfigWidget: Loading graph config for slot '{}' ===",
            slot_name
        );
        log::info!(
            "    text_overlay has {} lines, field_ids: {:?}",
            current_graph_config.text_overlay.len(),
            current_graph_config.text_overlay.iter().map(|l| l.field_id.as_str()).collect::<Vec<_>>()
        );
        log::info!(
            "    lcars_fields count: {}, field_ids: {:?}",
            lcars_fields.len(),
            lcars_fields.iter().map(|f| f.id.as_str()).collect::<Vec<_>>()
        );
        graph_widget.set_config(current_graph_config);

        // Set up change callback to sync config back
        let slot_name_clone = slot_name.to_string();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let graph_widget_rc = Rc::new(graph_widget);
        let graph_widget_for_callback = graph_widget_rc.clone();
        graph_widget_rc.set_on_change(move || {
            let graph_config = graph_widget_for_callback.get_config();
            log::info!(
                "=== LcarsConfigWidget: graph on_change for slot '{}', text_overlay has {} lines ===",
                slot_name_clone,
                graph_config.text_overlay.len()
            );
            if !graph_config.text_overlay.is_empty() {
                for (i, line) in graph_config.text_overlay.iter().enumerate() {
                    log::info!("    text_overlay[{}]: field_id='{}', font='{}', size={}",
                        i, line.field_id, line.font_family, line.font_size);
                }
            }
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

        // === Text Configuration Section (for Text display type) ===
        // Shows only text-related settings without bar-specific options
        let text_config_frame = gtk4::Frame::new(Some("Text Configuration"));
        text_config_frame.set_margin_top(12);

        // Use TextLineConfigWidget for text-only display configuration
        let text_widget = crate::ui::TextLineConfigWidget::new(lcars_fields.clone());

        // Initialize with current config if exists (convert from bar_config's text_overlay)
        let current_text_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.bar_config.text_overlay.text_config.clone())
                .unwrap_or_default()
        };
        text_widget.set_config(current_text_config);

        // Set up change callback to sync text config back to bar_config's text_overlay
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
            // Update the text_config in text_overlay and ensure overlay is enabled for Text mode
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

        // Create CoreBarsConfigWidget
        let core_bars_widget = CoreBarsConfigWidget::new();

        // Initialize with current config if exists
        let current_core_bars_config = {
            let cfg = config.borrow();
            cfg.frame.content_items
                .get(slot_name)
                .map(|item| item.core_bars_config.clone())
                .unwrap_or_default()
        };
        core_bars_widget.set_config(current_core_bars_config);

        // Set up change callback to sync config back
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

        core_bars_config_frame.set_child(Some(core_bars_widget_rc.widget()));
        inner_box.append(&core_bars_config_frame);

        // Show/hide config sections based on display type
        // Bar config: only for Bar and LevelBar (shows bar style + colors + text overlay)
        // Text config: only for Text (shows only text lines)
        // Graph config: only for Graph
        // Core Bars config: only for CoreBars
        let show_bar = matches!(current_type, ContentDisplayType::Bar | ContentDisplayType::LevelBar);
        let show_text = current_type == ContentDisplayType::Text;
        bar_config_frame.set_visible(show_bar);
        text_config_frame.set_visible(show_text);
        graph_config_frame.set_visible(current_type == ContentDisplayType::Graph);
        core_bars_config_frame.set_visible(current_type == ContentDisplayType::CoreBars);

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
        type_dropdown.connect_selected_notify(move |dropdown| {
            let display_type = match dropdown.selected() {
                0 => ContentDisplayType::Bar,
                1 => ContentDisplayType::Text,
                2 => ContentDisplayType::Graph,
                _ => ContentDisplayType::CoreBars,
            };
            // Show appropriate config for each display type
            let show_bar = matches!(display_type, ContentDisplayType::Bar | ContentDisplayType::LevelBar);
            let show_text = display_type == ContentDisplayType::Text;
            bar_config_frame_clone.set_visible(show_bar);
            text_config_frame_clone.set_visible(show_text);
            graph_config_frame_clone.set_visible(display_type == ContentDisplayType::Graph);
            core_bars_config_frame_clone.set_visible(display_type == ContentDisplayType::CoreBars);
            let mut cfg = config_clone.borrow_mut();
            let item = cfg.frame.content_items
                .entry(slot_name_clone.clone())
                .or_default();
            item.display_as = display_type;
            drop(cfg);
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        // Item height
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

    fn create_split_page(
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        split_widgets_out: &Rc<RefCell<Option<SplitWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Info label
        let info_label = Label::new(Some("Configure how groups are arranged and the dividers between them."));
        info_label.set_halign(gtk4::Align::Start);
        info_label.set_wrap(true);
        page.append(&info_label);

        // Layout Orientation (how groups are arranged)
        let orient_box = GtkBox::new(Orientation::Horizontal, 6);
        orient_box.append(&Label::new(Some("Layout:")));
        let orient_list = StringList::new(&["Vertical (side by side)", "Horizontal (stacked)"]);
        let orient_dropdown = DropDown::new(Some(orient_list), None::<gtk4::Expression>);
        let orient_idx = match config.borrow().frame.layout_orientation {
            SplitOrientation::Vertical => 0,
            SplitOrientation::Horizontal => 1,
        };
        orient_dropdown.set_selected(orient_idx);
        orient_dropdown.set_hexpand(true);
        orient_box.append(&orient_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let split_widgets_clone = split_widgets_out.clone();
        orient_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.layout_orientation = match dropdown.selected() {
                0 => SplitOrientation::Vertical,
                _ => SplitOrientation::Horizontal,
            };
            // Update sync checkbox sensitivity when layout changes
            if let Some(ref widgets) = *split_widgets_clone.borrow() {
                Self::update_sync_checkbox_sensitivity(&widgets.sync_segments_check, &config_clone);
            }
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&orient_box);

        // Divider section header
        let divider_header = Label::new(Some("Divider Settings (between groups)"));
        divider_header.set_halign(gtk4::Align::Start);
        divider_header.add_css_class("heading");
        divider_header.set_margin_top(12);
        page.append(&divider_header);

        // Divider width
        let divider_box = GtkBox::new(Orientation::Horizontal, 6);
        divider_box.append(&Label::new(Some("Divider Width:")));
        let divider_spin = SpinButton::with_range(2.0, 30.0, 2.0);
        divider_spin.set_value(config.borrow().frame.divider_config.width);
        divider_spin.set_hexpand(true);
        divider_box.append(&divider_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        divider_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.divider_config.width = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&divider_box);

        // Divider color
        let div_color_box = GtkBox::new(Orientation::Horizontal, 6);
        div_color_box.append(&Label::new(Some("Divider Color:")));
        let div_color_widget = Rc::new(ColorButtonWidget::new(config.borrow().frame.divider_config.color));
        div_color_box.append(div_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        div_color_widget.set_on_change(move |color| {
            config_clone.borrow_mut().frame.divider_config.color = color;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        page.append(&div_color_box);

        // Cap styles
        let start_cap_box = GtkBox::new(Orientation::Horizontal, 6);
        start_cap_box.append(&Label::new(Some("Start Cap:")));
        let start_cap_list = StringList::new(&["Square", "Round"]);
        let start_cap_dropdown = DropDown::new(Some(start_cap_list), None::<gtk4::Expression>);
        let start_cap_idx = match config.borrow().frame.divider_config.cap_start {
            DividerCapStyle::Square => 0,
            DividerCapStyle::Round => 1,
        };
        start_cap_dropdown.set_selected(start_cap_idx);
        start_cap_dropdown.set_hexpand(true);
        start_cap_box.append(&start_cap_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        start_cap_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.divider_config.cap_start = match dropdown.selected() {
                0 => DividerCapStyle::Square,
                _ => DividerCapStyle::Round,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&start_cap_box);

        let end_cap_box = GtkBox::new(Orientation::Horizontal, 6);
        end_cap_box.append(&Label::new(Some("End Cap:")));
        let end_cap_list = StringList::new(&["Square", "Round"]);
        let end_cap_dropdown = DropDown::new(Some(end_cap_list), None::<gtk4::Expression>);
        let end_cap_idx = match config.borrow().frame.divider_config.cap_end {
            DividerCapStyle::Square => 0,
            DividerCapStyle::Round => 1,
        };
        end_cap_dropdown.set_selected(end_cap_idx);
        end_cap_dropdown.set_hexpand(true);
        end_cap_box.append(&end_cap_dropdown);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        end_cap_dropdown.connect_selected_notify(move |dropdown| {
            config_clone.borrow_mut().frame.divider_config.cap_end = match dropdown.selected() {
                0 => DividerCapStyle::Square,
                _ => DividerCapStyle::Round,
            };
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&end_cap_box);

        // Divider spacing (padding before and after)
        let spacing_box = GtkBox::new(Orientation::Horizontal, 6);
        spacing_box.append(&Label::new(Some("Padding Before:")));
        let spacing_before_spin = SpinButton::with_range(0.0, 100.0, 5.0);
        spacing_before_spin.set_value(config.borrow().frame.divider_config.spacing_before);
        spacing_before_spin.set_width_chars(4);
        spacing_box.append(&spacing_before_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        spacing_before_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.divider_config.spacing_before = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });

        spacing_box.append(&Label::new(Some("After:")));
        let spacing_after_spin = SpinButton::with_range(0.0, 100.0, 5.0);
        spacing_after_spin.set_value(config.borrow().frame.divider_config.spacing_after);
        spacing_after_spin.set_width_chars(4);
        spacing_box.append(&spacing_after_spin);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        spacing_after_spin.connect_value_changed(move |spin| {
            config_clone.borrow_mut().frame.divider_config.spacing_after = spin.value();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&spacing_box);

        // Sync segments with groups checkbox
        let sync_segments_check = CheckButton::with_label("Sync segment heights with group heights");
        sync_segments_check.set_active(config.borrow().frame.sync_segments_to_groups);
        sync_segments_check.set_margin_top(12);

        // Update sensitivity based on conditions (layout=Horizontal, segment_count==group_count)
        let can_sync = {
            let cfg = config.borrow();
            cfg.frame.layout_orientation == SplitOrientation::Horizontal
                && cfg.frame.segment_count == cfg.frame.group_count
                && cfg.frame.group_count > 0
        };
        sync_segments_check.set_sensitive(can_sync);
        if !can_sync {
            sync_segments_check.set_tooltip_text(Some("Requires: Layout = Horizontal (stacked) and Segment count = Group count"));
        } else {
            sync_segments_check.set_tooltip_text(None);
        }

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        sync_segments_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.sync_segments_to_groups = check.is_active();
            Self::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&sync_segments_check);

        // Group Sizes section header
        let group_sizes_header = Label::new(Some("Group Sizes (relative weight)"));
        group_sizes_header.set_halign(gtk4::Align::Start);
        group_sizes_header.add_css_class("heading");
        group_sizes_header.set_margin_top(12);
        page.append(&group_sizes_header);

        let group_sizes_info = Label::new(Some("Set relative size weight for each group. Higher weight = larger size."));
        group_sizes_info.set_halign(gtk4::Align::Start);
        group_sizes_info.add_css_class("dim-label");
        page.append(&group_sizes_info);

        // Container for group size weight spinners (rebuilt dynamically)
        let group_weights_box = GtkBox::new(Orientation::Vertical, 4);
        group_weights_box.set_margin_top(4);
        Self::rebuild_group_weight_spinners(&group_weights_box, config, on_change, preview);
        page.append(&group_weights_box);

        // Store widget references for updating when config changes
        *split_widgets_out.borrow_mut() = Some(SplitWidgets {
            orient_dropdown: orient_dropdown.clone(),
            divider_spin: divider_spin.clone(),
            div_color_widget: div_color_widget.clone(),
            start_cap_dropdown: start_cap_dropdown.clone(),
            end_cap_dropdown: end_cap_dropdown.clone(),
            group_weights_box: group_weights_box.clone(),
            sync_segments_check: sync_segments_check.clone(),
        });

        page
    }

    /// Update the sync segments checkbox sensitivity based on current config
    fn update_sync_checkbox_sensitivity(
        check: &CheckButton,
        config: &Rc<RefCell<LcarsDisplayConfig>>,
    ) {
        let cfg = config.borrow();
        let can_sync = cfg.frame.layout_orientation == SplitOrientation::Horizontal
            && cfg.frame.segment_count == cfg.frame.group_count
            && cfg.frame.group_count > 0;
        check.set_sensitive(can_sync);
        if !can_sync {
            check.set_tooltip_text(Some("Requires: Layout = Horizontal (stacked) and Segment count = Group count"));
        } else {
            check.set_tooltip_text(None);
        }
    }

    /// Rebuild the group weight spinners based on current group count
    fn rebuild_group_weight_spinners(
        container: &GtkBox,
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) {
        // Clear existing children
        while let Some(child) = container.first_child() {
            container.remove(&child);
        }

        let cfg = config.borrow();
        let group_count = cfg.frame.group_count as usize;

        if group_count == 0 {
            let placeholder = Label::new(Some("No groups configured. Add sources in the Data Source tab."));
            placeholder.set_halign(gtk4::Align::Start);
            placeholder.add_css_class("dim-label");
            container.append(&placeholder);
            return;
        }

        // Create a spinner for each group
        for group_idx in 0..group_count {
            let group_num = group_idx + 1;
            let row = GtkBox::new(Orientation::Horizontal, 6);

            let label = Label::new(Some(&format!("Group {} Weight:", group_num)));
            label.set_width_chars(15);
            row.append(&label);

            let weight_spin = SpinButton::with_range(0.1, 10.0, 0.1);
            let current_weight = cfg.frame.group_size_weights.get(group_idx).copied().unwrap_or(1.0);
            weight_spin.set_value(current_weight);
            weight_spin.set_digits(1);
            weight_spin.set_hexpand(true);
            row.append(&weight_spin);

            let config_clone = config.clone();
            let on_change_clone = on_change.clone();
            let preview_clone = preview.clone();
            weight_spin.connect_value_changed(move |spin| {
                let mut cfg = config_clone.borrow_mut();
                // Ensure the weights vector is long enough
                while cfg.frame.group_size_weights.len() <= group_idx {
                    cfg.frame.group_size_weights.push(1.0);
                }
                cfg.frame.group_size_weights[group_idx] = spin.value();
                drop(cfg);
                Self::queue_redraw(&preview_clone, &on_change_clone);
            });

            container.append(&row);
        }
    }

    fn create_animation_page(
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        animation_widgets_out: &Rc<RefCell<Option<AnimationWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        Self::set_page_margins(&page);

        // Enable animation
        let enable_check = CheckButton::with_label("Enable Bar Animation");
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
        let speed_scale = Scale::with_range(Orientation::Horizontal, 1.0, 20.0, 1.0);
        speed_scale.set_value(config.borrow().animation_speed);
        speed_scale.set_hexpand(true);
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

        // Speed explanation
        let note_label = Label::new(Some(
            "Animation speed controls how quickly bar values\n\
             lerp toward their target. Higher = faster."
        ));
        note_label.set_halign(gtk4::Align::Start);
        note_label.set_margin_top(12);
        page.append(&note_label);

        // Store widget references for updating when config changes
        *animation_widgets_out.borrow_mut() = Some(AnimationWidgets {
            enable_check: enable_check.clone(),
            speed_scale: speed_scale.clone(),
        });

        page
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn set_config(&self, new_config: LcarsDisplayConfig) {
        // Debug: Log the font values being loaded
        log::debug!(
            "LCARS set_config - top font: '{}', bottom font: '{}'",
            new_config.frame.top_header.font,
            new_config.frame.bottom_header.font
        );
        // Debug: Log text_overlay for each content item
        for (slot_name, item) in &new_config.frame.content_items {
            log::debug!(
                "LcarsConfigWidget::set_config - slot '{}' has {} text_overlay lines in graph_config",
                slot_name,
                item.graph_config.text_overlay.len()
            );
        }

        // First update the internal config with the new values
        // Handle empty fonts by substituting defaults
        let mut config_to_use = new_config.clone();
        if config_to_use.frame.top_header.font.is_empty() {
            config_to_use.frame.top_header.font = "Sans".to_string();
        }
        if config_to_use.frame.bottom_header.font.is_empty() {
            config_to_use.frame.bottom_header.font = "Sans".to_string();
        }

        // Update internal config
        *self.config.borrow_mut() = config_to_use.clone();

        // Update frame widgets to reflect the new config
        if let Some(ref widgets) = *self.frame_widgets.borrow() {
            widgets.sidebar_spin.set_value(new_config.frame.sidebar_width);

            // Update sidebar position dropdown
            let pos_idx = match new_config.frame.sidebar_position {
                SidebarPosition::Left => 0,
                SidebarPosition::Right => 1,
            };
            widgets.pos_dropdown.set_selected(pos_idx);

            // Update extension mode dropdown
            let ext_idx = match new_config.frame.extension_mode {
                ExtensionMode::Top => 0,
                ExtensionMode::Bottom => 1,
                ExtensionMode::Both => 2,
                ExtensionMode::None => 3,
            };
            widgets.ext_dropdown.set_selected(ext_idx);

            widgets.top_spin.set_value(new_config.frame.top_bar_height);
            widgets.bottom_spin.set_value(new_config.frame.bottom_bar_height);
            widgets.corner_spin.set_value(new_config.frame.corner_radius);

            // Update extension corner style dropdown
            let ext_corner_idx = match new_config.frame.extension_corner_style {
                CornerStyle::Square => 0,
                CornerStyle::Round => 1,
            };
            widgets.ext_corner_dropdown.set_selected(ext_corner_idx);

            // Update content background color widget
            widgets.content_color_widget.set_color(new_config.frame.content_bg_color);

            widgets.padding_spin.set_value(new_config.frame.content_padding);
        }

        // Update headers widgets
        if let Some(ref widgets) = *self.headers_widgets.borrow() {
            // Top header
            widgets.top_show_check.set_active(config_to_use.frame.top_header.position == HeaderPosition::Top);
            widgets.top_text_entry.set_text(&config_to_use.frame.top_header.text);
            let top_shape_idx = match config_to_use.frame.top_header.shape {
                HeaderShape::Pill => 0,
                HeaderShape::Square => 1,
            };
            widgets.top_shape_dropdown.set_selected(top_shape_idx);
            widgets.top_bg_widget.set_color(config_to_use.frame.top_header.bg_color);
            widgets.top_text_color_widget.set_color(config_to_use.frame.top_header.text_color);
            widgets.top_font_btn.set_label(&format!(
                "{} {:.0}",
                config_to_use.frame.top_header.font,
                config_to_use.frame.top_header.font_size
            ));
            widgets.top_font_size_spin.set_value(config_to_use.frame.top_header.font_size);
            widgets.top_bold_check.set_active(config_to_use.frame.top_header.font_bold);
            let top_align_idx = match config_to_use.frame.top_header.align {
                HeaderAlign::Left => 0,
                HeaderAlign::Center => 1,
                HeaderAlign::Right => 2,
            };
            widgets.top_align_dropdown.set_selected(top_align_idx);

            // Bottom header
            widgets.bottom_show_check.set_active(config_to_use.frame.bottom_header.position == HeaderPosition::Bottom);
            widgets.bottom_text_entry.set_text(&config_to_use.frame.bottom_header.text);
            let bottom_shape_idx = match config_to_use.frame.bottom_header.shape {
                HeaderShape::Pill => 0,
                HeaderShape::Square => 1,
            };
            widgets.bottom_shape_dropdown.set_selected(bottom_shape_idx);
            widgets.bottom_bg_widget.set_color(config_to_use.frame.bottom_header.bg_color);
            widgets.bottom_text_color_widget.set_color(config_to_use.frame.bottom_header.text_color);
            widgets.bottom_font_btn.set_label(&format!(
                "{} {:.0}",
                config_to_use.frame.bottom_header.font,
                config_to_use.frame.bottom_header.font_size
            ));
            widgets.bottom_font_size_spin.set_value(config_to_use.frame.bottom_header.font_size);
            widgets.bottom_bold_check.set_active(config_to_use.frame.bottom_header.font_bold);
            let bottom_align_idx = match config_to_use.frame.bottom_header.align {
                HeaderAlign::Left => 0,
                HeaderAlign::Center => 1,
                HeaderAlign::Right => 2,
            };
            widgets.bottom_align_dropdown.set_selected(bottom_align_idx);
        } else {
            log::warn!("LCARS headers_widgets not available when setting config");
        }

        // Update segments widgets
        if let Some(ref widgets) = *self.segments_widgets.borrow() {
            widgets.count_spin.set_value(new_config.frame.segment_count as f64);

            // Show/hide segment frames
            let frames = widgets.segment_frames.borrow();
            for (i, frame) in frames.iter().enumerate() {
                frame.set_visible(i < new_config.frame.segment_count as usize);
            }

            // Update individual segment widgets
            let segment_widgets = widgets.segment_widgets.borrow();
            for (i, (label_entry, color_widget, label_color_widget, weight_spin, font_btn, size_spin)) in segment_widgets.iter().enumerate() {
                if let Some(seg) = new_config.frame.segments.get(i) {
                    label_entry.set_text(&seg.label);
                    color_widget.set_color(seg.color);
                    label_color_widget.set_color(seg.label_color);
                    weight_spin.set_value(seg.height_weight);
                    font_btn.set_label(&format!("{} {:.0}", seg.font, seg.font_size));
                    size_spin.set_value(seg.font_size);
                }
            }
        }

        // Update content widgets
        if let Some(ref widgets) = *self.content_widgets.borrow() {
            widgets.spacing_spin.set_value(new_config.frame.item_spacing);
        }

        // Update layout widgets
        if let Some(ref widgets) = *self.split_widgets.borrow() {
            let orient_idx = match new_config.frame.layout_orientation {
                SplitOrientation::Vertical => 0,
                SplitOrientation::Horizontal => 1,
            };
            widgets.orient_dropdown.set_selected(orient_idx);
            widgets.divider_spin.set_value(new_config.frame.divider_config.width);
            widgets.div_color_widget.set_color(new_config.frame.divider_config.color);
            let start_cap_idx = match new_config.frame.divider_config.cap_start {
                DividerCapStyle::Square => 0,
                DividerCapStyle::Round => 1,
            };
            widgets.start_cap_dropdown.set_selected(start_cap_idx);
            let end_cap_idx = match new_config.frame.divider_config.cap_end {
                DividerCapStyle::Square => 0,
                DividerCapStyle::Round => 1,
            };
            widgets.end_cap_dropdown.set_selected(end_cap_idx);
        }

        // Update animation widgets
        if let Some(ref widgets) = *self.animation_widgets.borrow() {
            widgets.enable_check.set_active(new_config.animation_enabled);
            widgets.speed_scale.set_value(new_config.animation_speed);
        }

        *self.config.borrow_mut() = new_config;
        self.preview.queue_draw();
    }

    pub fn get_config(&self) -> LcarsDisplayConfig {
        let config = self.config.borrow().clone();
        // Debug log text_overlay for each content item
        for (slot_name, item) in &config.frame.content_items {
            if !item.graph_config.text_overlay.is_empty() {
                log::debug!(
                    "LcarsConfigWidget::get_config - slot '{}' has {} text_overlay lines",
                    slot_name,
                    item.graph_config.text_overlay.len()
                );
            }
        }
        config
    }

    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Update the source summaries and rebuild the content notebook tabs
    /// Call this when the combo source configuration changes
    /// summaries: Vec of (slot_name, source_summary, group_num, item_idx)
    pub fn set_source_summaries(&self, summaries: Vec<(String, String, usize, u32)>) {
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
        let group_counts: Vec<u32> = group_nums.iter()
            .map(|n| *group_item_counts.get(n).unwrap_or(&0))
            .collect();

        // Update the frame config with group information
        {
            let mut cfg = self.config.borrow_mut();
            let new_group_count = group_nums.len();
            cfg.frame.group_count = new_group_count as u32;
            cfg.frame.group_item_counts = group_counts;

            // Ensure group_size_weights has the right length
            while cfg.frame.group_size_weights.len() < new_group_count {
                cfg.frame.group_size_weights.push(1.0);
            }
            // Trim if we have fewer groups now
            cfg.frame.group_size_weights.truncate(new_group_count);
        }

        *self.source_summaries.borrow_mut() = summaries;
        Self::rebuild_content_notebook_tabs(
            &self.content_notebook,
            &self.source_summaries,
            &self.config,
            &self.on_change,
            &self.preview,
            &self.available_fields,
        );

        // Rebuild group weight spinners and update sync checkbox in Layout tab if available
        if let Some(ref widgets) = *self.split_widgets.borrow() {
            Self::rebuild_group_weight_spinners(
                &widgets.group_weights_box,
                &self.config,
                &self.on_change,
                &self.preview,
            );
            Self::update_sync_checkbox_sensitivity(&widgets.sync_segments_check, &self.config);
        }

        // Queue preview redraw
        self.preview.queue_draw();
    }

    /// Update the available fields and rebuild the content notebook tabs
    /// Call this when the source configuration changes (e.g., combo source is reconfigured)
    pub fn set_available_fields(&self, fields: Vec<FieldMetadata>) {
        *self.available_fields.borrow_mut() = fields;
        // Rebuild tabs to use new fields
        Self::rebuild_content_notebook_tabs(
            &self.content_notebook,
            &self.source_summaries,
            &self.config,
            &self.on_change,
            &self.preview,
            &self.available_fields,
        );
    }
}

impl Default for LcarsConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
