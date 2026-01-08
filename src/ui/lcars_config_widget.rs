//! LCARS Combo configuration widget
//!
//! Provides a tabbed interface for configuring all aspects of the LCARS display.

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, DrawingArea, DropDown, Entry, Label, Notebook, Orientation,
    Scale, SpinButton, StringList,
};
use std::cell::RefCell;
use std::rc::Rc;

use crate::ui::clipboard::CLIPBOARD;
use crate::ui::color_button_widget::ColorButtonWidget;
use crate::ui::theme_color_selector::ThemeColorSelector;
use crate::ui::theme::{ColorSource, FontSource};
use crate::ui::theme_font_selector::ThemeFontSelector;
use crate::ui::lcars_display::{
    render_lcars_frame, render_content_background, SidebarPosition,
    ExtensionMode, CornerStyle, HeaderPosition, HeaderShape, HeaderAlign, SegmentConfig,
    DividerCapStyle, SplitOrientation,
};
use crate::ui::background::Color;
use crate::displayers::LcarsDisplayConfig;
use crate::core::FieldMetadata;
use crate::ui::combo_config_base;
use crate::ui::widget_builder::{ConfigWidgetBuilder, create_section_header};

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
    padding_top_spin: SpinButton,
    padding_left_spin: SpinButton,
    padding_right_spin: SpinButton,
    padding_bottom_spin: SpinButton,
}

/// Holds references to Headers tab widgets for updating when config changes
struct HeadersWidgets {
    // Notebook for Top/Bottom tabs
    headers_notebook: Notebook,
    top_tab_label: Label,
    bottom_tab_label: Label,
    // Top header
    top_show_check: CheckButton,
    top_text_entry: Entry,
    top_shape_dropdown: DropDown,
    top_bg_widget: Rc<ThemeColorSelector>,
    top_text_color_widget: Rc<ThemeColorSelector>,
    top_font_selector: Rc<ThemeFontSelector>,
    top_bold_check: CheckButton,
    top_align_dropdown: DropDown,
    top_height_spin: SpinButton,
    top_width_spin: SpinButton,
    // Bottom header
    bottom_show_check: CheckButton,
    bottom_text_entry: Entry,
    bottom_shape_dropdown: DropDown,
    bottom_bg_widget: Rc<ThemeColorSelector>,
    bottom_text_color_widget: Rc<ThemeColorSelector>,
    bottom_font_selector: Rc<ThemeFontSelector>,
    bottom_bold_check: CheckButton,
    bottom_align_dropdown: DropDown,
    bottom_height_spin: SpinButton,
    bottom_width_spin: SpinButton,
}

/// Holds references to Segments tab widgets for updating when config changes
struct SegmentsWidgets {
    count_spin: SpinButton,
    /// Notebook containing per-segment tabs
    segments_notebook: Notebook,
    /// Store segment widget refs: (label_entry, color_widget, label_color_widget, weight_spin, font_selector)
    segment_widgets: Rc<RefCell<Vec<(Entry, Rc<ThemeColorSelector>, Rc<ThemeColorSelector>, SpinButton, Rc<ThemeFontSelector>)>>>,
}

/// Holds references to Content tab widgets for updating when config changes
struct ContentWidgets {
    spacing_spin: SpinButton,
}

/// Holds references to Layout tab widgets for updating when config changes
struct SplitWidgets {
    orient_dropdown: DropDown,
    divider_spin: SpinButton,
    div_color_widget: Rc<ThemeColorSelector>,
    start_cap_dropdown: DropDown,
    end_cap_dropdown: DropDown,
    /// Container for combined group settings (weight + orientation per group)
    group_settings_box: GtkBox,
    /// Checkbox for syncing segments with groups
    sync_segments_check: CheckButton,
}

/// Holds references to Animation tab widgets for updating when config changes
struct AnimationWidgets {
    enable_check: CheckButton,
    speed_scale: Scale,
}

/// Holds references to Theme tab widgets for updating when config changes
struct ThemeWidgets {
    common: combo_config_base::CommonThemeWidgets,
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
    /// Theme tab widgets
    #[allow(dead_code)]
    theme_widgets: Rc<RefCell<Option<ThemeWidgets>>>,
    /// Callbacks to refresh theme reference sections when theme changes
    #[allow(dead_code)] // Kept for Rc ownership; callbacks are invoked via clones
    theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
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
        let theme_widgets: Rc<RefCell<Option<ThemeWidgets>>> = Rc::new(RefCell::new(None));
        let theme_ref_refreshers: Rc<RefCell<Vec<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(Vec::new()));

        // Preview at the top
        let preview = DrawingArea::new();
        preview.set_content_height(200);
        preview.set_hexpand(true);
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

        // Theme reference section - placed under preview for easy access from all tabs
        let (theme_ref_section, main_theme_refresh_cb) = combo_config_base::create_theme_reference_section(
            &config,
            |cfg| cfg.frame.theme.clone(),
        );
        theme_ref_refreshers.borrow_mut().push(main_theme_refresh_cb);

        // Create notebook for tabbed interface
        let notebook = Notebook::new();
        notebook.set_vexpand(true);

        // Tab 1: Theme (first for easy access)
        let theme_page = Self::create_theme_page(&config, &on_change, &preview, &theme_widgets, &theme_ref_refreshers);
        notebook.append_page(&theme_page, Some(&Label::new(Some("Theme"))));

        // Tab 2: Frame
        let frame_page = Self::create_frame_page(&config, &on_change, &preview, &frame_widgets, &theme_ref_refreshers);
        notebook.append_page(&frame_page, Some(&Label::new(Some("Frame"))));

        // Tab 3: Headers
        let headers_page = Self::create_headers_page(&config, &on_change, &preview, &headers_widgets, &theme_ref_refreshers);
        notebook.append_page(&headers_page, Some(&Label::new(Some("Headers"))));

        // Tab 4: Segments
        let segments_page = Self::create_segments_page(&config, &on_change, &preview, &segments_widgets, &split_widgets, &theme_ref_refreshers);
        notebook.append_page(&segments_page, Some(&Label::new(Some("Segments"))));

        // Tab 5: Content - with dynamic per-slot notebook
        let content_notebook = Rc::new(RefCell::new(Notebook::new()));
        let content_page = Self::create_content_page(&config, &on_change, &preview, &content_notebook, &source_summaries, &content_widgets, &available_fields, &theme_ref_refreshers);
        notebook.append_page(&content_page, Some(&Label::new(Some("Content"))));

        // Tab 6: Layout
        let split_page = Self::create_split_page(&config, &on_change, &preview, &split_widgets);
        notebook.append_page(&split_page, Some(&Label::new(Some("Layout"))));

        // Tab 7: Animation
        let animation_page = Self::create_animation_page(&config, &on_change, &animation_widgets);
        notebook.append_page(&animation_page, Some(&Label::new(Some("Animation"))));

        container.append(&preview);
        container.append(&theme_ref_section);
        container.append(&notebook);

        // Connect extension dropdown to update headers tab visibility
        {
            let headers_widgets_clone = headers_widgets.clone();
            if let Some(ref frame_w) = *frame_widgets.borrow() {
                frame_w.ext_dropdown.connect_selected_notify(move |dropdown| {
                    let ext_mode = match dropdown.selected() {
                        0 => ExtensionMode::Top,
                        1 => ExtensionMode::Bottom,
                        2 => ExtensionMode::Both,
                        _ => ExtensionMode::None,
                    };
                    if let Some(ref hw) = *headers_widgets_clone.borrow() {
                        Self::update_headers_tab_visibility(
                            &hw.top_tab_label,
                            &hw.bottom_tab_label,
                            ext_mode,
                        );
                    }
                });
            }
        }

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
            theme_widgets,
            theme_ref_refreshers,
        }
    }

    fn create_theme_page(
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        theme_widgets_out: &Rc<RefCell<Option<ThemeWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        // Create common theme widgets using the shared helper
        let config_for_change = config.clone();
        let on_change_for_redraw = on_change.clone();
        let preview_for_redraw = preview.clone();
        let refreshers_for_redraw = theme_ref_refreshers.clone();

        let common = combo_config_base::create_common_theme_widgets(
            &page,
            &config.borrow().frame.theme,
            move |mutator| {
                mutator(&mut config_for_change.borrow_mut().frame.theme);
            },
            move || {
                combo_config_base::queue_redraw(&preview_for_redraw, &on_change_for_redraw);
                combo_config_base::refresh_theme_refs(&refreshers_for_redraw);
            },
        );

        // Store widget refs
        *theme_widgets_out.borrow_mut() = Some(ThemeWidgets { common });

        page
    }

    fn create_frame_page(
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        frame_widgets_out: &Rc<RefCell<Option<FrameWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        // Theme reference section
        let (theme_ref_section, theme_refresh_cb) = combo_config_base::create_theme_reference_section(
            config,
            |cfg| cfg.frame.theme.clone(),
        );
        theme_ref_refreshers.borrow_mut().push(theme_refresh_cb);
        page.append(&theme_ref_section);

        let builder = ConfigWidgetBuilder::new(config, preview, on_change);

        let sidebar_spin = builder.spin_row(
            &page, "Sidebar Width:", 50.0, 300.0, 5.0, config.borrow().frame.sidebar_width,
            |cfg, v| cfg.frame.sidebar_width = v,
        );

        let pos_idx = match config.borrow().frame.sidebar_position {
            SidebarPosition::Left => 0,
            SidebarPosition::Right => 1,
        };
        let pos_dropdown = builder.dropdown_row(
            &page, "Sidebar Position:", &["Left", "Right"], pos_idx,
            |cfg, idx| cfg.frame.sidebar_position = if idx == 0 { SidebarPosition::Left } else { SidebarPosition::Right },
        );

        let ext_idx = match config.borrow().frame.extension_mode {
            ExtensionMode::Top => 0,
            ExtensionMode::Bottom => 1,
            ExtensionMode::Both => 2,
            ExtensionMode::None => 3,
        };
        let ext_dropdown = builder.dropdown_row(
            &page, "Extensions:", &["Top", "Bottom", "Both", "None"], ext_idx,
            |cfg, idx| cfg.frame.extension_mode = match idx {
                0 => ExtensionMode::Top,
                1 => ExtensionMode::Bottom,
                2 => ExtensionMode::Both,
                _ => ExtensionMode::None,
            },
        );

        let top_spin = builder.spin_row(
            &page, "Top Bar Height:", 20.0, 100.0, 2.0, config.borrow().frame.top_bar_height,
            |cfg, v| cfg.frame.top_bar_height = v,
        );

        let bottom_spin = builder.spin_row(
            &page, "Bottom Bar Height:", 20.0, 100.0, 2.0, config.borrow().frame.bottom_bar_height,
            |cfg, v| cfg.frame.bottom_bar_height = v,
        );

        let corner_spin = builder.spin_row(
            &page, "Corner Radius:", 0.0, 100.0, 5.0, config.borrow().frame.corner_radius,
            |cfg, v| cfg.frame.corner_radius = v,
        );

        let ext_corner_idx = match config.borrow().frame.extension_corner_style {
            CornerStyle::Square => 0,
            CornerStyle::Round => 1,
        };
        let ext_corner_dropdown = builder.dropdown_row(
            &page, "Extension Corners:", &["Square", "Round"], ext_corner_idx,
            |cfg, idx| cfg.frame.extension_corner_style = if idx == 0 { CornerStyle::Square } else { CornerStyle::Round },
        );

        // Content background color (keep manual - uses ColorButtonWidget directly)
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
            if let Some(cb) = on_change_clone.borrow().as_ref() { cb(); }
        });
        page.append(&content_color_box);

        // Content padding section
        page.append(&create_section_header("Content Padding"));

        let padding_spin = builder.spin_row(
            &page, "Overall:", 0.0, 50.0, 1.0, config.borrow().frame.content_padding,
            |cfg, v| cfg.frame.content_padding = v,
        );

        let padding_top_spin = builder.spin_row(
            &page, "Top:", -50.0, 50.0, 1.0, config.borrow().frame.content_padding_top,
            |cfg, v| cfg.frame.content_padding_top = v,
        );

        let padding_left_spin = builder.spin_row(
            &page, "Left:", -50.0, 50.0, 1.0, config.borrow().frame.content_padding_left,
            |cfg, v| cfg.frame.content_padding_left = v,
        );

        let padding_right_spin = builder.spin_row(
            &page, "Right:", -50.0, 50.0, 1.0, config.borrow().frame.content_padding_right,
            |cfg, v| cfg.frame.content_padding_right = v,
        );

        let padding_bottom_spin = builder.spin_row(
            &page, "Bottom:", -50.0, 50.0, 1.0, config.borrow().frame.content_padding_bottom,
            |cfg, v| cfg.frame.content_padding_bottom = v,
        );

        // Store widget references
        *frame_widgets_out.borrow_mut() = Some(FrameWidgets {
            sidebar_spin,
            pos_dropdown,
            ext_dropdown,
            top_spin,
            bottom_spin,
            corner_spin,
            ext_corner_dropdown,
            content_color_widget,
            padding_spin,
            padding_top_spin,
            padding_left_spin,
            padding_right_spin,
            padding_bottom_spin,
        });

        page
    }

    fn create_headers_page(
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        headers_widgets_out: &Rc<RefCell<Option<HeadersWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        // Theme reference section for quick access to theme colors/fonts
        let (theme_ref_section, theme_refresh_cb) = combo_config_base::create_theme_reference_section(
            config,
            |cfg| cfg.frame.theme.clone(),
        );
        theme_ref_refreshers.borrow_mut().push(theme_refresh_cb);
        page.append(&theme_ref_section);

        // Create notebook for Top/Bottom header tabs
        let headers_notebook = Notebook::new();
        headers_notebook.set_vexpand(true);

        // ===== TOP HEADER TAB =====
        let top_page = GtkBox::new(Orientation::Vertical, 8);
        top_page.set_margin_top(8);
        top_page.set_margin_bottom(8);
        top_page.set_margin_start(8);
        top_page.set_margin_end(8);

        // Top header show toggle
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        top_page.append(&top_show_check);

        // Top header text
        let top_text_box = GtkBox::new(Orientation::Horizontal, 6);
        top_text_box.append(&Label::new(Some("Text:")));
        let top_text_entry = Entry::new();
        top_text_entry.set_text(&config.borrow().frame.top_header.text);
        top_text_entry.set_hexpand(true);
        top_text_box.append(&top_text_entry);

        let top_copy_text_btn = Button::with_label("Copy");
        let top_paste_text_btn = Button::with_label("Paste");
        top_text_box.append(&top_copy_text_btn);
        top_text_box.append(&top_paste_text_btn);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_text_entry.connect_changed(move |entry| {
            config_clone.borrow_mut().frame.top_header.text = entry.text().to_string();
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        let top_text_entry_clone = top_text_entry.clone();
        top_copy_text_btn.connect_clicked(move |_| {
            let text = top_text_entry_clone.text().to_string();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_text(text);
            }
        });

        let top_text_entry_clone = top_text_entry.clone();
        top_paste_text_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(text) = clipboard.paste_text() {
                    top_text_entry_clone.set_text(&text);
                }
            }
        });
        top_page.append(&top_text_box);

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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        top_page.append(&top_shape_box);

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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        top_page.append(&top_size_box);

        // Top header colors row
        let top_colors_box = GtkBox::new(Orientation::Horizontal, 6);
        top_colors_box.append(&Label::new(Some("Background:")));
        let top_bg_widget = Rc::new(ThemeColorSelector::new(config.borrow().frame.top_header.bg_color.clone()));
        top_bg_widget.set_theme_config(config.borrow().frame.theme.clone());
        top_colors_box.append(top_bg_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_bg_widget.set_on_change(move |color_source| {
            config_clone.borrow_mut().frame.top_header.bg_color = color_source;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        top_colors_box.append(&Label::new(Some("Text:")));
        let top_text_color_widget = Rc::new(ThemeColorSelector::new(config.borrow().frame.top_header.text_color.clone()));
        top_text_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        top_colors_box.append(top_text_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_text_color_widget.set_on_change(move |color_source| {
            config_clone.borrow_mut().frame.top_header.text_color = color_source;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        top_page.append(&top_colors_box);

        // Top header font settings
        let top_font_box = GtkBox::new(Orientation::Horizontal, 6);
        top_font_box.append(&Label::new(Some("Font:")));

        let top_font_selector = Rc::new(ThemeFontSelector::new(config.borrow().frame.top_header.font.clone()));
        top_font_selector.set_theme_config(config.borrow().frame.theme.clone());
        top_font_box.append(top_font_selector.widget());

        let top_bold_check = CheckButton::with_label("Bold");
        top_bold_check.set_active(config.borrow().frame.top_header.font_bold);
        top_font_box.append(&top_bold_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_bold_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.top_header.font_bold = check.is_active();
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        let top_copy_font_btn = Button::with_label("Copy");
        let top_paste_font_btn = Button::with_label("Paste");
        top_font_box.append(&top_copy_font_btn);
        top_font_box.append(&top_paste_font_btn);

        let config_clone = config.clone();
        top_copy_font_btn.connect_clicked(move |_| {
            let cfg = config_clone.borrow();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_font_source(cfg.frame.top_header.font.clone(), cfg.frame.top_header.font_bold, false);
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let top_font_selector_clone = top_font_selector.clone();
        top_paste_font_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some((source, _bold, _italic)) = clipboard.paste_font_source() {
                    config_clone.borrow_mut().frame.top_header.font = source.clone();
                    top_font_selector_clone.set_source(source);
                    combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
                }
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        top_font_selector.set_on_change(move |source| {
            config_clone.borrow_mut().frame.top_header.font = source;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        top_page.append(&top_font_box);

        // Top header alignment
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        top_page.append(&top_align_box);

        // Add top page to notebook
        let top_tab_label = Label::new(Some("Top"));
        let _top_page_num = headers_notebook.append_page(&top_page, Some(&top_tab_label));

        // ===== BOTTOM HEADER TAB =====
        let bottom_page = GtkBox::new(Orientation::Vertical, 8);
        bottom_page.set_margin_top(8);
        bottom_page.set_margin_bottom(8);
        bottom_page.set_margin_start(8);
        bottom_page.set_margin_end(8);

        // Bottom header show toggle
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        bottom_page.append(&bottom_show_check);

        // Bottom header text
        let bottom_text_box = GtkBox::new(Orientation::Horizontal, 6);
        bottom_text_box.append(&Label::new(Some("Text:")));
        let bottom_text_entry = Entry::new();
        bottom_text_entry.set_text(&config.borrow().frame.bottom_header.text);
        bottom_text_entry.set_hexpand(true);
        bottom_text_box.append(&bottom_text_entry);

        let bottom_copy_text_btn = Button::with_label("Copy");
        let bottom_paste_text_btn = Button::with_label("Paste");
        bottom_text_box.append(&bottom_copy_text_btn);
        bottom_text_box.append(&bottom_paste_text_btn);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_text_entry.connect_changed(move |entry| {
            config_clone.borrow_mut().frame.bottom_header.text = entry.text().to_string();
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        let bottom_text_entry_clone = bottom_text_entry.clone();
        bottom_copy_text_btn.connect_clicked(move |_| {
            let text = bottom_text_entry_clone.text().to_string();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_text(text);
            }
        });

        let bottom_text_entry_clone = bottom_text_entry.clone();
        bottom_paste_text_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some(text) = clipboard.paste_text() {
                    bottom_text_entry_clone.set_text(&text);
                }
            }
        });
        bottom_page.append(&bottom_text_box);

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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        bottom_page.append(&bottom_shape_box);

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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        bottom_page.append(&bottom_size_box);

        // Bottom header colors row
        let bottom_colors_box = GtkBox::new(Orientation::Horizontal, 6);
        bottom_colors_box.append(&Label::new(Some("Background:")));
        let bottom_bg_widget = Rc::new(ThemeColorSelector::new(config.borrow().frame.bottom_header.bg_color.clone()));
        bottom_bg_widget.set_theme_config(config.borrow().frame.theme.clone());
        bottom_colors_box.append(bottom_bg_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_bg_widget.set_on_change(move |color_source| {
            config_clone.borrow_mut().frame.bottom_header.bg_color = color_source;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        bottom_colors_box.append(&Label::new(Some("Text:")));
        let bottom_text_color_widget = Rc::new(ThemeColorSelector::new(config.borrow().frame.bottom_header.text_color.clone()));
        bottom_text_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        bottom_colors_box.append(bottom_text_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_text_color_widget.set_on_change(move |color_source| {
            config_clone.borrow_mut().frame.bottom_header.text_color = color_source;
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });
        bottom_page.append(&bottom_colors_box);

        // Bottom header font settings
        let bottom_font_box = GtkBox::new(Orientation::Horizontal, 6);
        bottom_font_box.append(&Label::new(Some("Font:")));

        let bottom_font_selector = Rc::new(ThemeFontSelector::new(config.borrow().frame.bottom_header.font.clone()));
        bottom_font_selector.set_theme_config(config.borrow().frame.theme.clone());
        bottom_font_box.append(bottom_font_selector.widget());

        let bottom_bold_check = CheckButton::with_label("Bold");
        bottom_bold_check.set_active(config.borrow().frame.bottom_header.font_bold);
        bottom_font_box.append(&bottom_bold_check);

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_bold_check.connect_toggled(move |check| {
            config_clone.borrow_mut().frame.bottom_header.font_bold = check.is_active();
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });

        let bottom_copy_font_btn = Button::with_label("Copy");
        let bottom_paste_font_btn = Button::with_label("Paste");
        bottom_font_box.append(&bottom_copy_font_btn);
        bottom_font_box.append(&bottom_paste_font_btn);

        let config_clone = config.clone();
        bottom_copy_font_btn.connect_clicked(move |_| {
            let cfg = config_clone.borrow();
            if let Ok(mut clipboard) = CLIPBOARD.lock() {
                clipboard.copy_font_source(cfg.frame.bottom_header.font.clone(), cfg.frame.bottom_header.font_bold, false);
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let bottom_font_selector_clone = bottom_font_selector.clone();
        bottom_paste_font_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some((source, _bold, _italic)) = clipboard.paste_font_source() {
                    config_clone.borrow_mut().frame.bottom_header.font = source.clone();
                    bottom_font_selector_clone.set_source(source);
                    combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
                }
            }
        });

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        bottom_font_selector.set_on_change(move |source| {
            config_clone.borrow_mut().frame.bottom_header.font = source;
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        bottom_page.append(&bottom_font_box);

        // Bottom header alignment
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        bottom_page.append(&bottom_align_box);

        // Add bottom page to notebook
        let bottom_tab_label = Label::new(Some("Bottom"));
        let _bottom_page_num = headers_notebook.append_page(&bottom_page, Some(&bottom_tab_label));

        // Set initial tab visibility based on extension mode
        Self::update_headers_tab_visibility(&top_tab_label, &bottom_tab_label, config.borrow().frame.extension_mode);

        page.append(&headers_notebook);

        // Store widget references for updating when config changes
        *headers_widgets_out.borrow_mut() = Some(HeadersWidgets {
            headers_notebook: headers_notebook.clone(),
            top_tab_label: top_tab_label.clone(),
            bottom_tab_label: bottom_tab_label.clone(),
            top_show_check: top_show_check.clone(),
            top_text_entry: top_text_entry.clone(),
            top_shape_dropdown: top_shape_dropdown.clone(),
            top_bg_widget: top_bg_widget.clone(),
            top_text_color_widget: top_text_color_widget.clone(),
            top_font_selector: top_font_selector.clone(),
            top_bold_check: top_bold_check.clone(),
            top_align_dropdown: top_align_dropdown.clone(),
            top_height_spin: top_height_spin.clone(),
            top_width_spin: top_width_spin.clone(),
            bottom_show_check: bottom_show_check.clone(),
            bottom_text_entry: bottom_text_entry.clone(),
            bottom_shape_dropdown: bottom_shape_dropdown.clone(),
            bottom_bg_widget: bottom_bg_widget.clone(),
            bottom_text_color_widget: bottom_text_color_widget.clone(),
            bottom_font_selector: bottom_font_selector.clone(),
            bottom_bold_check: bottom_bold_check.clone(),
            bottom_align_dropdown: bottom_align_dropdown.clone(),
            bottom_height_spin: bottom_height_spin.clone(),
            bottom_width_spin: bottom_width_spin.clone(),
        });

        page
    }

    /// Update the visibility of header tabs based on extension mode.
    /// - Top only: show only Top tab
    /// - Bottom only: show only Bottom tab
    /// - Both or None: show both tabs
    fn update_headers_tab_visibility(top_tab_label: &Label, bottom_tab_label: &Label, extension_mode: ExtensionMode) {
        let show_top = matches!(extension_mode, ExtensionMode::Top | ExtensionMode::Both | ExtensionMode::None);
        let show_bottom = matches!(extension_mode, ExtensionMode::Bottom | ExtensionMode::Both | ExtensionMode::None);

        // Set visibility on the tab labels themselves
        top_tab_label.set_visible(show_top);
        bottom_tab_label.set_visible(show_bottom);
    }

    fn create_segments_page(
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        segments_widgets_out: &Rc<RefCell<Option<SegmentsWidgets>>>,
        split_widgets: &Rc<RefCell<Option<SplitWidgets>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

        // Theme reference section for quick access to theme colors/fonts
        let (theme_ref_section, theme_refresh_cb) = combo_config_base::create_theme_reference_section(
            config,
            |cfg| cfg.frame.theme.clone(),
        );
        theme_ref_refreshers.borrow_mut().push(theme_refresh_cb);
        page.append(&theme_ref_section);

        // Segment count
        let count_box = GtkBox::new(Orientation::Horizontal, 6);
        count_box.append(&Label::new(Some("Number of Segments:")));
        let count_spin = SpinButton::with_range(0.0, 10.0, 1.0);
        count_spin.set_value(config.borrow().frame.segment_count as f64);
        count_spin.set_hexpand(true);
        count_box.append(&count_spin);
        page.append(&count_box);

        // Create notebook for per-segment tabs
        let segments_notebook = Notebook::new();
        segments_notebook.set_vexpand(true);
        segments_notebook.set_scrollable(true);

        // Store per-segment widget refs: (label_entry, color_widget, label_color_widget, weight_spin, font_selector)
        let segment_widgets: Rc<RefCell<Vec<(Entry, Rc<ThemeColorSelector>, Rc<ThemeColorSelector>, SpinButton, Rc<ThemeFontSelector>)>>> = Rc::new(RefCell::new(Vec::new()));

        // Build initial segment tabs
        let initial_count = config.borrow().frame.segment_count as usize;
        Self::rebuild_segment_tabs(
            &segments_notebook,
            initial_count,
            config,
            on_change,
            preview,
            &segment_widgets,
        );

        // Connect count spin to rebuild segment tabs
        let segments_notebook_clone = segments_notebook.clone();
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let split_widgets_clone = split_widgets.clone();
        let segment_widgets_clone = segment_widgets.clone();
        count_spin.connect_value_changed(move |spin| {
            let count = spin.value() as usize;

            // Update config
            {
                let mut cfg = config_clone.borrow_mut();
                cfg.frame.segment_count = count as u32;

                // Ensure we have enough segments in config
                while cfg.frame.segments.len() < count {
                    cfg.frame.segments.push(SegmentConfig::default());
                }
            }

            // Rebuild the segment tabs
            Self::rebuild_segment_tabs(
                &segments_notebook_clone,
                count,
                &config_clone,
                &on_change_clone,
                &preview_clone,
                &segment_widgets_clone,
            );

            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);

            // Update sync checkbox sensitivity
            if let Some(ref widgets) = *split_widgets_clone.borrow() {
                Self::update_sync_checkbox_sensitivity(&widgets.sync_segments_check, &config_clone);
            }
        });

        page.append(&segments_notebook);

        // Store widget references for updating when config changes
        *segments_widgets_out.borrow_mut() = Some(SegmentsWidgets {
            count_spin: count_spin.clone(),
            segments_notebook: segments_notebook.clone(),
            segment_widgets: segment_widgets.clone(),
        });

        page
    }

    /// Rebuild segment tabs in the notebook based on segment count
    fn rebuild_segment_tabs(
        notebook: &Notebook,
        count: usize,
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        segment_widgets: &Rc<RefCell<Vec<(Entry, Rc<ThemeColorSelector>, Rc<ThemeColorSelector>, SpinButton, Rc<ThemeFontSelector>)>>>,
    ) {
        // Clear existing tabs
        while notebook.n_pages() > 0 {
            notebook.remove_page(Some(0));
        }
        segment_widgets.borrow_mut().clear();

        // Create tabs for each segment
        for seg_idx in 0..count {
            let (tab_content, widgets) = Self::create_segment_tab_content(seg_idx, config, on_change, preview);
            let tab_label = Label::new(Some(&format!("Seg {}", seg_idx + 1)));
            notebook.append_page(&tab_content, Some(&tab_label));
            segment_widgets.borrow_mut().push(widgets);
        }

        // Show placeholder if no segments
        if count == 0 {
            let placeholder = GtkBox::new(Orientation::Vertical, 8);
            placeholder.set_valign(gtk4::Align::Center);
            placeholder.set_halign(gtk4::Align::Center);
            let label = Label::new(Some("No segments configured.\nIncrease the segment count above to add segments."));
            label.add_css_class("dim-label");
            placeholder.append(&label);
            notebook.append_page(&placeholder, Some(&Label::new(Some("Info"))));
        }
    }

    /// Create content for a single segment tab
    fn create_segment_tab_content(
        seg_idx: usize,
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
    ) -> (GtkBox, (Entry, Rc<ThemeColorSelector>, Rc<ThemeColorSelector>, SpinButton, Rc<ThemeFontSelector>)) {
        let seg_box = GtkBox::new(Orientation::Vertical, 8);
        seg_box.set_margin_start(12);
        seg_box.set_margin_end(12);
        seg_box.set_margin_top(12);
        seg_box.set_margin_bottom(12);

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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        seg_box.append(&label_box);

        // Colors row (segment color + label color)
        let colors_box = GtkBox::new(Orientation::Horizontal, 12);
        colors_box.append(&Label::new(Some("Segment Color:")));
        let seg_color = config.borrow().frame.segments.get(seg_idx)
            .map(|s| s.color.clone())
            .unwrap_or_else(|| ColorSource::custom(Color::new(0.8, 0.4, 0.4, 1.0)));
        let color_widget = Rc::new(ThemeColorSelector::new(seg_color));
        color_widget.set_theme_config(config.borrow().frame.theme.clone());
        colors_box.append(color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        color_widget.set_on_change(move |color_source| {
            let mut cfg = config_clone.borrow_mut();
            while cfg.frame.segments.len() <= seg_idx {
                cfg.frame.segments.push(SegmentConfig::default());
            }
            cfg.frame.segments[seg_idx].color = color_source;
            drop(cfg);
            preview_clone.queue_draw();
            if let Some(cb) = on_change_clone.borrow().as_ref() {
                cb();
            }
        });

        colors_box.append(&Label::new(Some("Label Color:")));
        let label_color = config.borrow().frame.segments.get(seg_idx)
            .map(|s| s.label_color.clone())
            .unwrap_or_else(|| ColorSource::custom(Color::new(0.0, 0.0, 0.0, 1.0)));
        let label_color_widget = Rc::new(ThemeColorSelector::new(label_color));
        label_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        colors_box.append(label_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        label_color_widget.set_on_change(move |color_source| {
            let mut cfg = config_clone.borrow_mut();
            while cfg.frame.segments.len() <= seg_idx {
                cfg.frame.segments.push(SegmentConfig::default());
            }
            cfg.frame.segments[seg_idx].label_color = color_source;
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        seg_box.append(&weight_box);

        // Font settings (with theme font selector)
        let font_box = GtkBox::new(Orientation::Horizontal, 6);
        font_box.append(&Label::new(Some("Font:")));

        let font_source = {
            let cfg = config.borrow();
            if let Some(seg) = cfg.frame.segments.get(seg_idx) {
                seg.font.clone()
            } else {
                FontSource::default()
            }
        };

        let font_selector = Rc::new(ThemeFontSelector::new(font_source));
        font_selector.set_theme_config(config.borrow().frame.theme.clone());
        font_box.append(font_selector.widget());

        // Copy/Paste font buttons
        let copy_font_btn = Button::with_label("Copy");
        let paste_font_btn = Button::with_label("Paste");
        font_box.append(&copy_font_btn);
        font_box.append(&paste_font_btn);

        // Copy font handler
        let config_clone = config.clone();
        copy_font_btn.connect_clicked(move |_| {
            let cfg = config_clone.borrow();
            if let Some(seg) = cfg.frame.segments.get(seg_idx) {
                if let Ok(mut clipboard) = CLIPBOARD.lock() {
                    clipboard.copy_font_source(seg.font.clone(), false, false);
                }
            }
        });

        // Paste font handler
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        let font_selector_clone = font_selector.clone();
        paste_font_btn.connect_clicked(move |_| {
            if let Ok(clipboard) = CLIPBOARD.lock() {
                if let Some((source, _bold, _italic)) = clipboard.paste_font_source() {
                    {
                        let mut cfg = config_clone.borrow_mut();
                        while cfg.frame.segments.len() <= seg_idx {
                            cfg.frame.segments.push(SegmentConfig::default());
                        }
                        cfg.frame.segments[seg_idx].font = source.clone();
                    }
                    font_selector_clone.set_source(source);
                    combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
                }
            }
        });

        // Font selector change handler
        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        font_selector.set_on_change(move |source| {
            {
                let mut cfg = config_clone.borrow_mut();
                while cfg.frame.segments.len() <= seg_idx {
                    cfg.frame.segments.push(SegmentConfig::default());
                }
                cfg.frame.segments[seg_idx].font = source;
            }
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        seg_box.append(&font_box);

        (seg_box, (label_entry, color_widget.clone(), label_color_widget.clone(), weight_spin.clone(), font_selector.clone()))
    }

    fn create_content_page(
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        content_notebook: &Rc<RefCell<Notebook>>,
        source_summaries: &Rc<RefCell<Vec<(String, String, usize, u32)>>>,
        content_widgets_out: &Rc<RefCell<Option<ContentWidgets>>>,
        available_fields: &Rc<RefCell<Vec<FieldMetadata>>>,
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
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
        Self::rebuild_content_notebook_tabs(content_notebook, source_summaries, config, on_change, preview, available_fields, theme_ref_refreshers);

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
        theme_ref_refreshers: &Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    ) {
        combo_config_base::rebuild_content_tabs(
            config,
            on_change,
            preview,
            content_notebook,
            source_summaries,
            available_fields,
            |cfg| &cfg.frame.content_items,
            |cfg, slot_name, item| {
                cfg.frame.content_items.insert(slot_name.to_string(), item);
            },
            theme_ref_refreshers,
            |cfg| cfg.frame.theme.clone(),
        );
    }

    /// Create a theme reference section showing current theme colors and fonts with copy buttons
    #[allow(dead_code)]
    fn create_theme_reference_section(
        config: &Rc<RefCell<LcarsDisplayConfig>>,
    ) -> (gtk4::Frame, Rc<dyn Fn()>) {
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
                    clipboard.copy_font_source(FontSource::Theme { index: font_idx, size: 14.0 }, false, false);
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

    fn create_split_page(
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        preview: &DrawingArea,
        split_widgets_out: &Rc<RefCell<Option<SplitWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&divider_box);

        // Divider color
        let div_color_box = GtkBox::new(Orientation::Horizontal, 6);
        div_color_box.append(&Label::new(Some("Divider Color:")));
        let div_color_widget = Rc::new(ThemeColorSelector::new(config.borrow().frame.divider_config.color.clone()));
        div_color_widget.set_theme_config(config.borrow().frame.theme.clone());
        div_color_box.append(div_color_widget.widget());

        let config_clone = config.clone();
        let on_change_clone = on_change.clone();
        let preview_clone = preview.clone();
        div_color_widget.set_on_change(move |color_source| {
            config_clone.borrow_mut().frame.divider_config.color = color_source;
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
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
            combo_config_base::queue_redraw(&preview_clone, &on_change_clone);
        });
        page.append(&sync_segments_check);

        // Combined group settings section (weight + orientation per group)
        let group_settings_box = combo_config_base::create_combined_group_settings_section(&page);
        combo_config_base::rebuild_combined_group_settings(
            &group_settings_box,
            config,
            |c: &mut LcarsDisplayConfig| &mut c.frame,
            on_change,
            preview,
        );

        // Store widget references for updating when config changes
        *split_widgets_out.borrow_mut() = Some(SplitWidgets {
            orient_dropdown: orient_dropdown.clone(),
            divider_spin: divider_spin.clone(),
            div_color_widget: div_color_widget.clone(),
            start_cap_dropdown: start_cap_dropdown.clone(),
            end_cap_dropdown: end_cap_dropdown.clone(),
            group_settings_box: group_settings_box.clone(),
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

    fn create_animation_page(
        config: &Rc<RefCell<LcarsDisplayConfig>>,
        on_change: &Rc<RefCell<Option<Box<dyn Fn()>>>>,
        animation_widgets_out: &Rc<RefCell<Option<AnimationWidgets>>>,
    ) -> GtkBox {
        let page = GtkBox::new(Orientation::Vertical, 8);
        combo_config_base::set_page_margins(&page);

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
            "LCARS set_config - top font: '{:?}', bottom font: '{:?}'",
            new_config.frame.top_header.font,
            new_config.frame.bottom_header.font
        );
        // Debug: Log text_overlay for each content item
        for (slot_name, item) in &new_config.frame.content_items {
            log::debug!(
                "LcarsConfigWidget::set_config - slot '{}' has text_overlay enabled={}, lines={} in graph_config",
                slot_name,
                item.graph_config.text_overlay.enabled,
                item.graph_config.text_overlay.text_config.lines.len()
            );
        }

        // IMPORTANT: Temporarily disable on_change callback to prevent signal cascade.
        // When we call set_value() on widgets, their signal handlers fire and call on_change.
        // This causes redundant updates since we're setting the config directly anyway.
        let saved_callback = self.on_change.borrow_mut().take();

        // First update the internal config with the new values
        let config_to_use = new_config.clone();

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
            widgets.padding_top_spin.set_value(new_config.frame.content_padding_top);
            widgets.padding_left_spin.set_value(new_config.frame.content_padding_left);
            widgets.padding_right_spin.set_value(new_config.frame.content_padding_right);
            widgets.padding_bottom_spin.set_value(new_config.frame.content_padding_bottom);
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
            widgets.top_bg_widget.set_source(config_to_use.frame.top_header.bg_color.clone());
            widgets.top_text_color_widget.set_source(config_to_use.frame.top_header.text_color.clone());
            widgets.top_font_selector.set_source(config_to_use.frame.top_header.font.clone());
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
            widgets.bottom_bg_widget.set_source(config_to_use.frame.bottom_header.bg_color.clone());
            widgets.bottom_text_color_widget.set_source(config_to_use.frame.bottom_header.text_color.clone());
            widgets.bottom_font_selector.set_source(config_to_use.frame.bottom_header.font.clone());
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

            // Rebuild segment tabs with new config
            let count = new_config.frame.segment_count as usize;
            Self::rebuild_segment_tabs(
                &widgets.segments_notebook,
                count,
                &self.config,
                &self.on_change,
                &self.preview,
                &widgets.segment_widgets,
            );
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
            widgets.div_color_widget.set_source(new_config.frame.divider_config.color.clone());
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

        // Update theme widgets (fonts and colors)
        if let Some(ref widgets) = *self.theme_widgets.borrow() {
            widgets.common.color1_widget.set_color(new_config.frame.theme.color1);
            widgets.common.color2_widget.set_color(new_config.frame.theme.color2);
            widgets.common.color3_widget.set_color(new_config.frame.theme.color3);
            widgets.common.color4_widget.set_color(new_config.frame.theme.color4);
            widgets.common.gradient_editor.set_theme_config(new_config.frame.theme.clone());
            widgets.common.gradient_editor.set_gradient_source_config(&new_config.frame.theme.gradient);
            widgets.common.font1_btn.set_label(&new_config.frame.theme.font1_family);
            widgets.common.font1_size_spin.set_value(new_config.frame.theme.font1_size);
            widgets.common.font2_btn.set_label(&new_config.frame.theme.font2_family);
            widgets.common.font2_size_spin.set_value(new_config.frame.theme.font2_size);
        }

        *self.config.borrow_mut() = new_config;

        // Restore the on_change callback now that widget updates are complete
        *self.on_change.borrow_mut() = saved_callback;

        // Update Theme Reference section with new theme colors
        combo_config_base::refresh_theme_refs(&self.theme_ref_refreshers);

        self.preview.queue_draw();
    }

    pub fn get_config(&self) -> LcarsDisplayConfig {
        let config = self.config.borrow().clone();
        // Debug log text_overlay for each content item
        for (slot_name, item) in &config.frame.content_items {
            if item.graph_config.text_overlay.enabled && !item.graph_config.text_overlay.text_config.lines.is_empty() {
                log::debug!(
                    "LcarsConfigWidget::get_config - slot '{}' has {} text_overlay lines",
                    slot_name,
                    item.graph_config.text_overlay.text_config.lines.len()
                );
            }
        }
        config
    }

    /// Get a reference to the internal config Rc for use in callbacks
    pub fn get_config_rc(&self) -> Rc<RefCell<LcarsDisplayConfig>> {
        self.config.clone()
    }

    pub fn set_on_change<F: Fn() + 'static>(&self, callback: F) {
        *self.on_change.borrow_mut() = Some(Box::new(callback));
    }

    /// Set the theme configuration. Call this BEFORE set_config to ensure
    /// font selectors have the correct theme when the UI is rebuilt.
    pub fn set_theme(&self, theme: crate::ui::theme::ComboThemeConfig) {
        self.config.borrow_mut().frame.theme = theme.clone();
        // Update gradient editor with new theme colors
        if let Some(ref widgets) = *self.theme_widgets.borrow() {
            widgets.common.gradient_editor.set_theme_config(theme.clone());
        }
        // Update header widgets with new theme colors and fonts
        if let Some(ref widgets) = *self.headers_widgets.borrow() {
            widgets.top_bg_widget.set_theme_config(theme.clone());
            widgets.top_text_color_widget.set_theme_config(theme.clone());
            widgets.top_font_selector.set_theme_config(theme.clone());
            widgets.bottom_bg_widget.set_theme_config(theme.clone());
            widgets.bottom_text_color_widget.set_theme_config(theme.clone());
            widgets.bottom_font_selector.set_theme_config(theme.clone());
        }
        // Update segment widgets with new theme colors and fonts
        if let Some(ref widgets) = *self.segments_widgets.borrow() {
            for (_, color_widget, label_color_widget, _, font_selector) in widgets.segment_widgets.borrow().iter() {
                color_widget.set_theme_config(theme.clone());
                label_color_widget.set_theme_config(theme.clone());
                font_selector.set_theme_config(theme.clone());
            }
        }
        // Update layout widgets with new theme colors
        if let Some(ref widgets) = *self.split_widgets.borrow() {
            widgets.div_color_widget.set_theme_config(theme.clone());
        }
        // Trigger all theme refreshers to update child widgets
        for refresher in self.theme_ref_refreshers.borrow().iter() {
            refresher();
        }
        // Redraw the main preview to reflect theme color changes
        self.preview.queue_draw();
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
            &self.theme_ref_refreshers,
        );

        // Rebuild combined group settings and update sync checkbox in Layout tab if available
        if let Some(ref widgets) = *self.split_widgets.borrow() {
            combo_config_base::rebuild_combined_group_settings(
                &widgets.group_settings_box,
                &self.config,
                |c: &mut LcarsDisplayConfig| &mut c.frame,
                &self.on_change,
                &self.preview,
            );
            Self::update_sync_checkbox_sensitivity(&widgets.sync_segments_check, &self.config);
        }

        // Queue preview redraw
        self.preview.queue_draw();

        // Notify that config has changed so displayer gets updated
        if let Some(cb) = self.on_change.borrow().as_ref() {
            cb();
        }
    }

    /// Update the available fields for content configuration.
    /// NOTE: This only stores the fields - it does NOT rebuild tabs.
    /// Call set_source_summaries() after this to trigger the rebuild.
    /// This avoids double-rebuilding when both methods are called together.
    pub fn set_available_fields(&self, fields: Vec<FieldMetadata>) {
        *self.available_fields.borrow_mut() = fields;
        // Don't rebuild here - set_source_summaries() will be called next and will rebuild
    }

    /// Extract transferable configuration (layout, content items, animation settings).
    /// This excludes theme-specific settings like colors, fonts, and frame styles.
    pub fn get_transferable_config(&self) -> crate::ui::combo_config_base::TransferableComboConfig {
        let config = self.config.borrow();
        crate::ui::combo_config_base::TransferableComboConfig {
            group_count: config.frame.group_count as usize,
            group_item_counts: config.frame.group_item_counts.clone(),
            group_size_weights: config.frame.group_size_weights.clone(),
            group_item_orientations: config.frame.group_item_orientations.clone(),
            layout_orientation: config.frame.layout_orientation,
            content_items: config.frame.content_items.clone(),
            content_padding: config.frame.content_padding,
            item_spacing: config.frame.item_spacing,
            animation_enabled: config.animation_enabled,
            animation_speed: config.animation_speed,
        }
    }

    /// Apply transferable configuration from another combo panel.
    /// This preserves theme-specific settings while updating layout and content.
    pub fn apply_transferable_config(&self, transfer: &crate::ui::combo_config_base::TransferableComboConfig) {
        {
            let mut config = self.config.borrow_mut();
            config.frame.group_count = transfer.group_count as u32;
            config.frame.group_item_counts = transfer.group_item_counts.clone();
            config.frame.group_size_weights = transfer.group_size_weights.clone();
            config.frame.group_item_orientations = transfer.group_item_orientations.clone();
            config.frame.layout_orientation = transfer.layout_orientation;
            config.frame.content_items = transfer.content_items.clone();
            config.frame.content_padding = transfer.content_padding;
            config.frame.item_spacing = transfer.item_spacing;
            config.animation_enabled = transfer.animation_enabled;
            config.animation_speed = transfer.animation_speed;
        }
        // Queue preview redraw
        self.preview.queue_draw();
    }
}

impl Default for LcarsConfigWidget {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
