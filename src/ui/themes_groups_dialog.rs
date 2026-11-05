//! Themes & Groups window.
//!
//! Phase 0 of the tree-based configuration redesign
//! (`docs/config-ui-redesign.md` §9.1). A tree of the theme library (global
//! theme, user themes, built-in presets), panel groups with their member
//! panels, and ungrouped panels, with an editor for the selected node on the
//! right.
//!
//! * Drag a panel onto a group (or onto "Ungrouped") to move it.
//! * Drag a theme onto a group or a panel to assign it.
//!
//! Edits are live: the resolved theme (panel → group → global) is pushed into
//! every affected panel immediately and the config is marked dirty.
//!
//! The stored `AppConfig.panels_v2` is refreshed from the live panels on
//! open and after every structural change, so the pure library operations on
//! `AppConfig` (rename, delete-in-use, membership) always see current data.

use crate::config::AppConfig;
use crate::core::Panel;
use crate::ui::theme::{ComboThemeConfig, ThemeRef};
use crate::ui::{GlobalThemeWidget, GridLayout};
use gtk4::prelude::*;
use gtk4::{
    gdk, gio, glib, Box as GtkBox, Button, DropDown, Entry, Label, ListView, Orientation, Paned,
    ScrolledWindow, SignalListItemFactory, SingleSelection, Stack, StringList, TreeExpander,
    TreeListModel, TreeListRow, Window,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

thread_local! {
    /// The single open window and the strong owner of its state. Closures
    /// only hold `Weak<State>`, so closing the window drops everything.
    static OPEN: RefCell<Option<(glib::WeakRef<Window>, Rc<State>)>> = const { RefCell::new(None) };
}

/// A row in the tree. Stored in a `glib::BoxedAnyObject` so the GTK list
/// model can carry it without a custom GObject subclass.
#[derive(Clone, Debug, PartialEq)]
enum Node {
    ThemesRoot,
    GlobalTheme,
    Theme { name: String, builtin: bool },
    GroupsRoot,
    Group { id: String },
    UngroupedRoot,
    Panel { id: String },
}

/// What a dragged row means when dropped.
enum DragPayload {
    Panel(String),
    Theme(ThemeRef),
}

impl Node {
    fn boxed(self) -> glib::BoxedAnyObject {
        glib::BoxedAnyObject::new(self)
    }

    fn from_object(obj: &glib::Object) -> Option<Node> {
        obj.downcast_ref::<glib::BoxedAnyObject>()
            .map(|b| b.borrow::<Node>().clone())
    }

    fn drag_payload(&self) -> Option<DragPayload> {
        match self {
            Node::Panel { id } => Some(DragPayload::Panel(id.clone())),
            Node::GlobalTheme => Some(DragPayload::Theme(ThemeRef::Global)),
            Node::Theme { name, .. } => Some(DragPayload::Theme(ThemeRef::named(name))),
            _ => None,
        }
    }
}

/// Entries of the panel page's theme dropdown, in display order.
#[derive(Clone, Debug, PartialEq)]
enum ThemeChoice {
    Inherit,
    Global,
    Named(String),
    Inline,
}

struct State {
    app_config: Rc<RefCell<AppConfig>>,
    grid_layout: Rc<RefCell<GridLayout>>,
    config_dirty: Arc<AtomicBool>,
    window_background: gtk4::DrawingArea,
    window: glib::WeakRef<Window>,
    root_store: gio::ListStore,
    selection: SingleSelection,
    stack: Stack,
    /// Set while editors are being loaded so their change callbacks do not
    /// write the values straight back.
    loading: Cell<bool>,
    panel_labels: Rc<RefCell<HashMap<String, String>>>,

    // Theme page
    theme_name_entry: Entry,
    theme_widget: GlobalThemeWidget,
    theme_users_label: Label,
    theme_delete_btn: Button,
    current_theme: RefCell<Option<Node>>,

    // Group page
    group_name_entry: Entry,
    group_theme_dropdown: DropDown,
    group_theme_names: RefCell<Vec<String>>,
    group_members_label: Label,
    current_group: RefCell<Option<String>>,

    // Panel page
    panel_info_label: Label,
    panel_theme_dropdown: DropDown,
    panel_theme_choices: RefCell<Vec<ThemeChoice>>,
    panel_inline_box: GtkBox,
    panel_inline_widget: GlobalThemeWidget,
    current_panel: RefCell<Option<String>>,
}

/// Show the Themes & Groups window, or present it if already open.
pub fn show_themes_groups_dialog(
    parent: &Window,
    app_config: &Rc<RefCell<AppConfig>>,
    grid_layout: &Rc<RefCell<GridLayout>>,
    config_dirty: &Arc<AtomicBool>,
    window_background: &gtk4::DrawingArea,
) {
    if let Some(existing) = OPEN.with(|o| o.borrow().as_ref().and_then(|(w, _)| w.upgrade())) {
        existing.present();
        return;
    }

    let window = Window::builder()
        .title("Themes & Groups")
        .transient_for(parent)
        .modal(false)
        .default_width(820)
        .default_height(600)
        .build();

    // ---- Tree model -------------------------------------------------------
    let root_store = gio::ListStore::new::<glib::BoxedAnyObject>();
    let panel_labels: Rc<RefCell<HashMap<String, String>>> = Rc::new(RefCell::new(HashMap::new()));
    let tree_model = {
        let app_config = app_config.clone();
        TreeListModel::new(root_store.clone(), false, true, move |obj| {
            let node = Node::from_object(obj)?;
            children_for(&node, &app_config.borrow())
        })
    };
    let selection = SingleSelection::new(Some(tree_model));
    selection.set_autoselect(false);

    // ---- Editor pages -----------------------------------------------------
    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);

    let empty_label = Label::new(Some("Select a theme, group or panel"));
    empty_label.add_css_class("dim-label");
    stack.add_named(&empty_label, Some("empty"));

    // Theme page
    let theme_page = page_box();
    let theme_name_row = GtkBox::new(Orientation::Horizontal, 8);
    theme_name_row.append(&Label::new(Some("Name:")));
    let theme_name_entry = Entry::new();
    theme_name_entry.set_hexpand(true);
    theme_name_row.append(&theme_name_entry);
    theme_page.append(&theme_name_row);
    let theme_widget = GlobalThemeWidget::new();
    let theme_scroll = ScrolledWindow::new();
    theme_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    theme_scroll.set_vexpand(true);
    theme_scroll.set_child(Some(theme_widget.widget()));
    theme_page.append(&theme_scroll);
    let theme_users_label = Label::new(None);
    theme_users_label.add_css_class("dim-label");
    theme_users_label.set_halign(gtk4::Align::Start);
    theme_users_label.set_wrap(true);
    theme_page.append(&theme_users_label);
    let theme_btn_row = GtkBox::new(Orientation::Horizontal, 6);
    let theme_duplicate_btn = Button::with_label("Duplicate");
    let theme_delete_btn = Button::with_label("Delete");
    theme_delete_btn.add_css_class("destructive-action");
    theme_btn_row.append(&theme_duplicate_btn);
    theme_btn_row.append(&theme_delete_btn);
    theme_page.append(&theme_btn_row);
    stack.add_named(&theme_page, Some("theme"));

    // Group page
    let group_page = page_box();
    let group_name_row = GtkBox::new(Orientation::Horizontal, 8);
    group_name_row.append(&Label::new(Some("Name:")));
    let group_name_entry = Entry::new();
    group_name_entry.set_hexpand(true);
    group_name_row.append(&group_name_entry);
    group_page.append(&group_name_row);
    let group_theme_row = GtkBox::new(Orientation::Horizontal, 8);
    group_theme_row.append(&Label::new(Some("Theme:")));
    let group_theme_dropdown = DropDown::new(None::<StringList>, None::<gtk4::Expression>);
    group_theme_dropdown.set_hexpand(true);
    group_theme_row.append(&group_theme_dropdown);
    group_page.append(&group_theme_row);
    let group_members_label = Label::new(None);
    group_members_label.add_css_class("dim-label");
    group_members_label.set_halign(gtk4::Align::Start);
    group_members_label.set_wrap(true);
    group_page.append(&group_members_label);
    let group_hint = Label::new(Some(
        "Drag panels from \"Ungrouped\" (or another group) onto this group to add them.\n\
         Drag a theme onto the group to assign it.",
    ));
    group_hint.add_css_class("dim-label");
    group_hint.set_halign(gtk4::Align::Start);
    group_hint.set_wrap(true);
    group_page.append(&group_hint);
    let group_delete_btn = Button::with_label("Delete Group");
    group_delete_btn.add_css_class("destructive-action");
    group_delete_btn.set_halign(gtk4::Align::Start);
    group_page.append(&group_delete_btn);
    stack.add_named(&group_page, Some("group"));

    // Panel page
    let panel_page = page_box();
    let panel_info_label = Label::new(None);
    panel_info_label.set_halign(gtk4::Align::Start);
    panel_info_label.set_wrap(true);
    panel_page.append(&panel_info_label);
    let panel_theme_row = GtkBox::new(Orientation::Horizontal, 8);
    panel_theme_row.append(&Label::new(Some("Theme:")));
    let panel_theme_dropdown = DropDown::new(None::<StringList>, None::<gtk4::Expression>);
    panel_theme_dropdown.set_hexpand(true);
    panel_theme_row.append(&panel_theme_dropdown);
    panel_page.append(&panel_theme_row);
    let panel_inline_box = GtkBox::new(Orientation::Vertical, 6);
    let panel_inline_heading = Label::new(Some("Inline theme (private to this panel)"));
    panel_inline_heading.add_css_class("heading");
    panel_inline_heading.set_halign(gtk4::Align::Start);
    panel_inline_box.append(&panel_inline_heading);
    let panel_inline_widget = GlobalThemeWidget::new();
    let panel_inline_scroll = ScrolledWindow::new();
    panel_inline_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    panel_inline_scroll.set_vexpand(true);
    panel_inline_scroll.set_child(Some(panel_inline_widget.widget()));
    panel_inline_box.append(&panel_inline_scroll);
    panel_page.append(&panel_inline_box);
    stack.add_named(&panel_page, Some("panel"));

    // ---- State ------------------------------------------------------------
    let state = Rc::new(State {
        app_config: app_config.clone(),
        grid_layout: grid_layout.clone(),
        config_dirty: config_dirty.clone(),
        window_background: window_background.clone(),
        window: window.downgrade(),
        root_store: root_store.clone(),
        selection: selection.clone(),
        stack: stack.clone(),
        loading: Cell::new(false),
        panel_labels: panel_labels.clone(),
        theme_name_entry: theme_name_entry.clone(),
        theme_widget,
        theme_users_label: theme_users_label.clone(),
        theme_delete_btn: theme_delete_btn.clone(),
        current_theme: RefCell::new(None),
        group_name_entry: group_name_entry.clone(),
        group_theme_dropdown: group_theme_dropdown.clone(),
        group_theme_names: RefCell::new(Vec::new()),
        group_members_label: group_members_label.clone(),
        current_group: RefCell::new(None),
        panel_info_label: panel_info_label.clone(),
        panel_theme_dropdown: panel_theme_dropdown.clone(),
        panel_theme_choices: RefCell::new(Vec::new()),
        panel_inline_box: panel_inline_box.clone(),
        panel_inline_widget,
        current_panel: RefCell::new(None),
    });
    let weak = Rc::downgrade(&state);

    // ---- Tree view --------------------------------------------------------
    let factory = SignalListItemFactory::new();
    {
        let weak = weak.clone();
        factory.connect_setup(move |_, item| {
            let Some(item) = item.downcast_ref::<gtk4::ListItem>() else {
                return;
            };
            let expander = TreeExpander::new();
            let hbox = GtkBox::new(Orientation::Horizontal, 8);
            let title = Label::new(None);
            title.set_xalign(0.0);
            title.set_hexpand(true);
            title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            let summary = Label::new(None);
            summary.add_css_class("dim-label");
            summary.set_xalign(1.0);
            hbox.append(&title);
            hbox.append(&summary);
            expander.set_child(Some(&hbox));
            item.set_child(Some(&expander));

            // Drag: the payload is read at drag time from whatever row is bound.
            let drag = gtk4::DragSource::new();
            drag.set_actions(gdk::DragAction::COPY);
            // Weak: the ListItem owns this widget's controller; a strong clone
            // would be a reference cycle kept alive after the window closes.
            let item_for_drag = item.downgrade();
            drag.connect_prepare(move |_, _, _| {
                let node = bound_node(&item_for_drag.upgrade()?)?;
                node.drag_payload()?;
                Some(gdk::ContentProvider::for_value(&node.boxed().to_value()))
            });
            hbox.add_controller(drag);

            let drop =
                gtk4::DropTarget::new(glib::BoxedAnyObject::static_type(), gdk::DragAction::COPY);
            let item_for_drop = item.downgrade();
            let weak = weak.clone();
            drop.connect_drop(move |_, value, _, _| {
                let Some(state) = weak.upgrade() else {
                    return false;
                };
                let Some(target) = item_for_drop.upgrade().and_then(|i| bound_node(&i)) else {
                    return false;
                };
                let Ok(obj) = value.get::<glib::BoxedAnyObject>() else {
                    return false;
                };
                let source = obj.borrow::<Node>().clone();
                state.handle_drop(source, target)
            });
            hbox.add_controller(drop);
        });
    }
    {
        let weak = weak.clone();
        factory.connect_bind(move |_, item| {
            let Some(item) = item.downcast_ref::<gtk4::ListItem>() else {
                return;
            };
            let Some(row) = item.item().and_then(|o| o.downcast::<TreeListRow>().ok()) else {
                return;
            };
            let Some(expander) = item.child().and_then(|c| c.downcast::<TreeExpander>().ok())
            else {
                return;
            };
            expander.set_list_row(Some(&row));
            let Some(hbox) = expander.child().and_then(|c| c.downcast::<GtkBox>().ok()) else {
                return;
            };
            let title = hbox.first_child().and_then(|c| c.downcast::<Label>().ok());
            let summary = hbox.last_child().and_then(|c| c.downcast::<Label>().ok());
            let Some(node) = row.item().and_then(|o| Node::from_object(&o)) else {
                return;
            };
            let (t, s) = match weak.upgrade() {
                Some(state) => state.row_text(&node),
                None => (String::new(), String::new()),
            };
            if let Some(title) = title {
                title.set_text(&t);
            }
            if let Some(summary) = summary {
                summary.set_text(&s);
            }
        });
    }
    factory.connect_unbind(|_, item| {
        if let Some(expander) = item
            .downcast_ref::<gtk4::ListItem>()
            .and_then(|i| i.child())
            .and_then(|c| c.downcast::<TreeExpander>().ok())
        {
            expander.set_list_row(None);
        }
    });

    let list_view = ListView::new(Some(selection.clone()), Some(factory));
    list_view.set_vexpand(true);
    let tree_scroll = ScrolledWindow::new();
    tree_scroll.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
    tree_scroll.set_child(Some(&list_view));
    tree_scroll.set_vexpand(true);

    let add_row = GtkBox::new(Orientation::Horizontal, 6);
    add_row.set_margin_top(6);
    let add_theme_btn = Button::with_label("+ Theme");
    let add_group_btn = Button::with_label("+ Group");
    add_row.append(&add_theme_btn);
    add_row.append(&add_group_btn);

    let left = GtkBox::new(Orientation::Vertical, 0);
    left.append(&tree_scroll);
    left.append(&add_row);
    left.set_margin_start(8);
    left.set_margin_top(8);
    left.set_margin_bottom(8);

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_start_child(Some(&left));
    paned.set_end_child(Some(&stack));
    paned.set_position(330);
    paned.set_shrink_start_child(false);
    paned.set_shrink_end_child(false);
    window.set_child(Some(&paned));

    // ---- Signals ----------------------------------------------------------
    {
        let weak = weak.clone();
        selection.connect_selected_notify(move |_| {
            if let Some(state) = weak.upgrade() {
                state.show_selected();
            }
        });
    }
    {
        let weak = weak.clone();
        add_theme_btn.connect_clicked(move |_| {
            if let Some(state) = weak.upgrade() {
                state.add_theme();
            }
        });
    }
    {
        let weak = weak.clone();
        add_group_btn.connect_clicked(move |_| {
            if let Some(state) = weak.upgrade() {
                state.add_group();
            }
        });
    }

    // Theme page
    {
        let weak = weak.clone();
        state.theme_widget.set_on_change(move || {
            if let Some(state) = weak.upgrade() {
                state.theme_edited();
            }
        });
    }
    {
        let weak = weak.clone();
        theme_name_entry.connect_activate(move |_| {
            if let Some(state) = weak.upgrade() {
                state.commit_theme_rename();
            }
        });
        let weak2 = Rc::downgrade(&state);
        let focus = gtk4::EventControllerFocus::new();
        focus.connect_leave(move |_| {
            if let Some(state) = weak2.upgrade() {
                state.commit_theme_rename();
            }
        });
        theme_name_entry.add_controller(focus);
    }
    {
        let weak = weak.clone();
        theme_duplicate_btn.connect_clicked(move |_| {
            if let Some(state) = weak.upgrade() {
                state.duplicate_theme();
            }
        });
    }
    {
        let weak = weak.clone();
        theme_delete_btn.connect_clicked(move |_| {
            if let Some(state) = weak.upgrade() {
                state.delete_theme();
            }
        });
    }

    // Group page
    {
        let weak = weak.clone();
        group_name_entry.connect_changed(move |entry| {
            if let Some(state) = weak.upgrade() {
                state.group_name_changed(entry.text().to_string());
            }
        });
        let weak2 = Rc::downgrade(&state);
        group_name_entry.connect_activate(move |_| {
            if let Some(state) = weak2.upgrade() {
                state.refresh_tree_keep_selection();
            }
        });
        let weak3 = Rc::downgrade(&state);
        let focus = gtk4::EventControllerFocus::new();
        focus.connect_leave(move |_| {
            if let Some(state) = weak3.upgrade() {
                state.refresh_tree_keep_selection();
            }
        });
        group_name_entry.add_controller(focus);
    }
    {
        let weak = weak.clone();
        group_theme_dropdown.connect_selected_notify(move |_| {
            if let Some(state) = weak.upgrade() {
                state.group_theme_selected();
            }
        });
    }
    {
        let weak = weak.clone();
        group_delete_btn.connect_clicked(move |_| {
            if let Some(state) = weak.upgrade() {
                state.delete_group();
            }
        });
    }

    // Panel page
    {
        let weak = weak.clone();
        panel_theme_dropdown.connect_selected_notify(move |_| {
            if let Some(state) = weak.upgrade() {
                state.panel_theme_selected();
            }
        });
    }
    {
        let weak = weak.clone();
        state.panel_inline_widget.set_on_change(move || {
            if let Some(state) = weak.upgrade() {
                state.panel_inline_edited();
            }
        });
    }

    window.connect_close_request(|w| {
        w.set_child(None::<&gtk4::Widget>);
        OPEN.with(|o| o.borrow_mut().take());
        glib::Propagation::Proceed
    });

    OPEN.with(|o| *o.borrow_mut() = Some((window.downgrade(), state.clone())));

    state.pull_from_live();
    state.rebuild_tree(None);
    state.stack.set_visible_child_name("empty");
    window.present();
}

fn page_box() -> GtkBox {
    let b = GtkBox::new(Orientation::Vertical, 10);
    b.set_margin_start(12);
    b.set_margin_end(12);
    b.set_margin_top(12);
    b.set_margin_bottom(12);
    b
}

/// The node currently bound to a list item, if any.
fn bound_node(item: &gtk4::ListItem) -> Option<Node> {
    item.item()?
        .downcast::<TreeListRow>()
        .ok()?
        .item()
        .and_then(|o| Node::from_object(&o))
}

/// Children of a tree node, read from the stored config.
fn children_for(node: &Node, cfg: &AppConfig) -> Option<gio::ListModel> {
    let store = gio::ListStore::new::<glib::BoxedAnyObject>();
    match node {
        Node::ThemesRoot => {
            store.append(&Node::GlobalTheme.boxed());
            for name in cfg.themes.keys() {
                store.append(
                    &Node::Theme {
                        name: name.clone(),
                        builtin: false,
                    }
                    .boxed(),
                );
            }
            for (name, _) in ComboThemeConfig::builtin_presets() {
                store.append(
                    &Node::Theme {
                        name: name.to_string(),
                        builtin: true,
                    }
                    .boxed(),
                );
            }
        }
        Node::GroupsRoot => {
            for group in &cfg.groups {
                store.append(
                    &Node::Group {
                        id: group.id.clone(),
                    }
                    .boxed(),
                );
            }
        }
        Node::Group { id } => {
            for pid in cfg.group_members(id) {
                store.append(&Node::Panel { id: pid }.boxed());
            }
        }
        Node::UngroupedRoot => {
            for panel in cfg.panels_v2.iter().filter(|p| p.group_id.is_none()) {
                store.append(
                    &Node::Panel {
                        id: panel.id.clone(),
                    }
                    .boxed(),
                );
            }
        }
        _ => return None,
    }
    Some(store.upcast())
}

impl State {
    fn panels(&self) -> Vec<Arc<RwLock<Panel>>> {
        self.grid_layout.borrow().get_panels()
    }

    fn window(&self) -> Option<Window> {
        self.window.upgrade()
    }

    /// Refresh the stored panels (theme, group, and everything else) and the
    /// row labels from the live panels.
    fn pull_from_live(&self) {
        let mut labels = HashMap::new();
        let mut data_list = Vec::new();
        for panel in self.panels() {
            let guard = panel.blocking_read();
            labels.insert(
                guard.id.clone(),
                format!(
                    "{} · {} @ {},{}",
                    guard.displayer.name(),
                    guard.source.metadata().name,
                    guard.geometry.x,
                    guard.geometry.y
                ),
            );
            data_list.push(guard.to_data());
        }
        self.app_config.borrow_mut().set_panels(data_list);
        *self.panel_labels.borrow_mut() = labels;
    }

    /// Resolve every live panel's theme (panel → group → global) from the
    /// panel's own `ThemeRef`/`group_id`, push it into the displayer, redraw,
    /// and mark the config dirty.
    ///
    /// The live panels are the truth. `AppConfig.panels_v2` is only a
    /// snapshot (refreshed by `pull_from_live`) for building the tree and for
    /// the library operations; it is never written back wholesale, so an
    /// edit made elsewhere while this window is open cannot be clobbered.
    fn push_resolved(&self) {
        {
            let cfg = self.app_config.borrow();
            for panel in self.panels() {
                let mut guard = panel.blocking_write();
                let theme = cfg.resolve_theme_for(&guard.theme_ref(), guard.group_id());
                if let Err(e) = guard.apply_resolved_theme(&theme) {
                    log::warn!("Failed to apply theme to panel {}: {}", guard.id, e);
                }
            }
        }
        self.grid_layout.borrow().queue_redraw_all_panels();
        self.window_background.queue_draw();
        self.config_dirty.store(true, Ordering::Relaxed);
    }

    /// Apply a change to one live panel.
    fn with_live_panel(&self, panel_id: &str, f: impl FnOnce(&mut Panel)) {
        for panel in self.panels() {
            if panel.blocking_read().id != panel_id {
                continue;
            }
            f(&mut panel.blocking_write());
            return;
        }
        log::warn!("Themes & Groups: panel {} no longer exists", panel_id);
    }

    fn live_theme_ref(&self, panel_id: &str) -> Option<ThemeRef> {
        self.panels().into_iter().find_map(|p| {
            let guard = p.blocking_read();
            (guard.id == panel_id).then(|| guard.theme_ref())
        })
    }
    fn rebuild_tree(&self, reselect: Option<Node>) {
        self.root_store.remove_all();
        for node in [Node::ThemesRoot, Node::GroupsRoot, Node::UngroupedRoot] {
            self.root_store.append(&node.boxed());
        }
        if let Some(target) = reselect {
            self.select_node(&target);
        }
    }

    fn refresh_tree_keep_selection(&self) {
        let current = self.selected_node();
        self.rebuild_tree(current);
    }

    fn selected_node(&self) -> Option<Node> {
        self.selection
            .selected_item()
            .and_then(|o| o.downcast::<TreeListRow>().ok())
            .and_then(|row| row.item())
            .and_then(|o| Node::from_object(&o))
    }

    fn select_node(&self, target: &Node) {
        let n = self.selection.n_items();
        for i in 0..n {
            let node = self
                .selection
                .item(i)
                .and_then(|o| o.downcast::<TreeListRow>().ok())
                .and_then(|row| row.item())
                .and_then(|o| Node::from_object(&o));
            if node.as_ref() == Some(target) {
                self.selection.set_selected(i);
                return;
            }
        }
        self.selection.set_selected(gtk4::INVALID_LIST_POSITION);
        self.stack.set_visible_child_name("empty");
    }

    /// Title and dim summary for a row.
    fn row_text(&self, node: &Node) -> (String, String) {
        let cfg = self.app_config.borrow();
        match node {
            Node::ThemesRoot => (
                "Themes".to_string(),
                format!(
                    "{} user, {} built-in",
                    cfg.themes.len(),
                    ComboThemeConfig::builtin_presets().len()
                ),
            ),
            Node::GlobalTheme => {
                let users = cfg
                    .panels_v2
                    .iter()
                    .filter(|p| matches!(p.theme, ThemeRef::Global))
                    .count();
                (
                    "Global".to_string(),
                    if users > 0 {
                        format!("{} panel(s)", users)
                    } else {
                        "default".to_string()
                    },
                )
            }
            Node::Theme { name, builtin } => {
                let (panels, groups) = cfg.theme_users(name);
                let mut parts = Vec::new();
                if *builtin {
                    parts.push("built-in".to_string());
                }
                if !panels.is_empty() {
                    parts.push(format!("{} panel(s)", panels.len()));
                }
                if !groups.is_empty() {
                    parts.push(format!("{} group(s)", groups.len()));
                }
                (name.clone(), parts.join(" · "))
            }
            Node::GroupsRoot => ("Groups".to_string(), format!("{}", cfg.groups.len())),
            Node::Group { id } => match cfg.group(id) {
                Some(group) => (
                    group.name.clone(),
                    format!(
                        "{} · theme: {}",
                        cfg.group_members(id).len(),
                        describe_group_theme(&group.theme)
                    ),
                ),
                None => ("(missing group)".to_string(), String::new()),
            },
            Node::UngroupedRoot => (
                "Ungrouped".to_string(),
                format!(
                    "{}",
                    cfg.panels_v2
                        .iter()
                        .filter(|p| p.group_id.is_none())
                        .count()
                ),
            ),
            Node::Panel { id } => {
                let label = self
                    .panel_labels
                    .borrow()
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| id.clone());
                let theme = cfg
                    .panels_v2
                    .iter()
                    .find(|p| &p.id == id)
                    .map(|p| p.theme.describe())
                    .unwrap_or_default();
                (label, theme)
            }
        }
    }

    // ---- Selection → editor -------------------------------------------------

    fn show_selected(&self) {
        let Some(node) = self.selected_node() else {
            self.stack.set_visible_child_name("empty");
            return;
        };
        // Pages read the stored snapshot; refresh it so they show live state.
        self.pull_from_live();
        self.loading.set(true);
        match &node {
            Node::GlobalTheme | Node::Theme { .. } => self.load_theme_page(&node),
            Node::Group { id } => self.load_group_page(id),
            Node::Panel { id } => self.load_panel_page(id),
            _ => self.stack.set_visible_child_name("empty"),
        }
        self.loading.set(false);
    }
    fn load_theme_page(&self, node: &Node) {
        let cfg = self.app_config.borrow();
        let (name, theme, builtin, users_text) = match node {
            Node::GlobalTheme => {
                let users = cfg
                    .panels_v2
                    .iter()
                    .filter(|p| matches!(p.theme, ThemeRef::Global))
                    .count();
                (
                    "Global".to_string(),
                    cfg.global_theme.clone(),
                    true,
                    format!(
                        "The global theme is used by every panel and group that does not pick another one ({} panel(s) reference it explicitly).",
                        users
                    ),
                )
            }
            Node::Theme { name, builtin } => {
                let theme = cfg
                    .theme_library()
                    .lookup_named(name)
                    .unwrap_or_else(|| cfg.global_theme.clone());
                let (panels, groups) = cfg.theme_users(name);
                let mut users = Vec::new();
                for pid in &panels {
                    users.push(
                        self.panel_labels
                            .borrow()
                            .get(pid)
                            .cloned()
                            .unwrap_or_else(|| pid.clone()),
                    );
                }
                for gid in &groups {
                    if let Some(g) = cfg.group(gid) {
                        users.push(format!("group \"{}\"", g.name));
                    }
                }
                let text = if *builtin {
                    "Built-in preset (read-only). Duplicate it to make an editable copy."
                        .to_string()
                } else if users.is_empty() {
                    "Not used by any panel or group.".to_string()
                } else {
                    format!("Used by: {}", users.join(", "))
                };
                (name.clone(), theme, *builtin, text)
            }
            _ => return,
        };
        drop(cfg);

        *self.current_theme.borrow_mut() = Some(node.clone());
        self.theme_name_entry.set_text(&name);
        // The global theme is editable but not renamable; built-ins are neither.
        self.theme_name_entry
            .set_sensitive(matches!(node, Node::Theme { builtin: false, .. }));
        self.theme_widget.set_config(theme);
        self.theme_widget
            .widget()
            .set_sensitive(!builtin || matches!(node, Node::GlobalTheme));
        self.theme_users_label.set_text(&users_text);
        let deletable = match node {
            Node::Theme {
                name,
                builtin: false,
            } => {
                let cfg = self.app_config.borrow();
                let (p, g) = cfg.theme_users(name);
                p.is_empty() && g.is_empty()
            }
            _ => false,
        };
        self.theme_delete_btn.set_sensitive(deletable);
        self.stack.set_visible_child_name("theme");
    }

    fn load_group_page(&self, id: &str) {
        let cfg = self.app_config.borrow();
        let Some(group) = cfg.group(id) else {
            self.stack.set_visible_child_name("empty");
            return;
        };
        *self.current_group.borrow_mut() = Some(id.to_string());
        self.group_name_entry.set_text(&group.name);

        // Dropdown: Global, then user themes, then built-ins
        let mut names = vec!["Global".to_string()];
        names.extend(cfg.themes.keys().cloned());
        names.extend(
            ComboThemeConfig::builtin_presets()
                .iter()
                .map(|(n, _)| n.to_string()),
        );
        let selected = match &group.theme {
            ThemeRef::Named { name } => names.iter().position(|n| n == name).unwrap_or(0),
            _ => 0,
        };
        let display: Vec<String> = names
            .iter()
            .enumerate()
            .map(|(i, n)| {
                if i == 0 {
                    "Global".to_string()
                } else if cfg.themes.contains_key(n) {
                    n.clone()
                } else {
                    format!("{} (built-in)", n)
                }
            })
            .collect();
        let refs: Vec<&str> = display.iter().map(|s| s.as_str()).collect();
        self.group_theme_dropdown
            .set_model(Some(&StringList::new(&refs)));
        self.group_theme_dropdown.set_selected(selected as u32);
        *self.group_theme_names.borrow_mut() = names;

        let members = cfg.group_members(id);
        let labels = self.panel_labels.borrow();
        let text = if members.is_empty() {
            "No members yet.".to_string()
        } else {
            format!(
                "Members: {}",
                members
                    .iter()
                    .map(|m| labels.get(m).cloned().unwrap_or_else(|| m.clone()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        self.group_members_label.set_text(&text);
        self.stack.set_visible_child_name("group");
    }

    fn load_panel_page(&self, id: &str) {
        let cfg = self.app_config.borrow();
        let Some(panel) = cfg.panels_v2.iter().find(|p| p.id == id) else {
            self.stack.set_visible_child_name("empty");
            return;
        };
        *self.current_panel.borrow_mut() = Some(id.to_string());

        let label = self
            .panel_labels
            .borrow()
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_string());
        let group_text = panel
            .group_id
            .as_deref()
            .and_then(|gid| cfg.group(gid))
            .map(|g| format!("Group: {}", g.name))
            .unwrap_or_else(|| "Not in a group (drag it onto a group to add it)".to_string());
        self.panel_info_label
            .set_text(&format!("{}\n{}", label, group_text));

        let mut choices = vec![ThemeChoice::Inherit, ThemeChoice::Global];
        let mut display = vec![
            "Inherit (group, else global)".to_string(),
            "Global".to_string(),
        ];
        for name in cfg.themes.keys() {
            choices.push(ThemeChoice::Named(name.clone()));
            display.push(name.clone());
        }
        for (name, _) in ComboThemeConfig::builtin_presets() {
            choices.push(ThemeChoice::Named(name.to_string()));
            display.push(format!("{} (built-in)", name));
        }
        choices.push(ThemeChoice::Inline);
        display.push("Inline (private to this panel)".to_string());

        let selected = match &panel.theme {
            ThemeRef::Inherit => 0,
            ThemeRef::Global => 1,
            ThemeRef::Named { name } => choices
                .iter()
                .position(|c| matches!(c, ThemeChoice::Named(n) if n == name))
                .unwrap_or(0),
            ThemeRef::Inline { .. } => choices.len() - 1,
        };
        let refs: Vec<&str> = display.iter().map(|s| s.as_str()).collect();
        self.panel_theme_dropdown
            .set_model(Some(&StringList::new(&refs)));
        self.panel_theme_dropdown.set_selected(selected as u32);
        *self.panel_theme_choices.borrow_mut() = choices;

        let inline = matches!(panel.theme, ThemeRef::Inline { .. });
        self.panel_inline_box.set_visible(inline);
        if let ThemeRef::Inline { theme } = &panel.theme {
            self.panel_inline_widget.set_config(theme.clone());
        }
        self.stack.set_visible_child_name("panel");
    }

    // ---- Theme page actions ---------------------------------------------------

    fn theme_edited(&self) {
        if self.loading.get() {
            return;
        }
        let new_theme = self.theme_widget.get_config();
        let current = self.current_theme.borrow().clone();
        {
            let mut cfg = self.app_config.borrow_mut();
            match current {
                Some(Node::GlobalTheme) => cfg.global_theme = new_theme,
                Some(Node::Theme {
                    name,
                    builtin: false,
                }) => {
                    let _ = cfg.set_theme(&name, new_theme);
                }
                _ => return,
            }
        }
        self.push_resolved();
    }

    fn commit_theme_rename(&self) {
        let Some(Node::Theme {
            name: old,
            builtin: false,
        }) = self.current_theme.borrow().clone()
        else {
            return;
        };
        let new = self.theme_name_entry.text().trim().to_string();
        if new == old {
            return;
        }
        self.pull_from_live();
        let result = self.app_config.borrow_mut().rename_theme(&old, &new);
        match result {
            Ok(()) => {
                // The stored snapshot was rewritten; do the same to the live panels.
                for panel in self.panels() {
                    let mut guard = panel.blocking_write();
                    if guard.theme_ref().named_name() == Some(old.as_str()) {
                        guard.set_theme_ref(ThemeRef::named(&new));
                    }
                }
                *self.current_theme.borrow_mut() = Some(Node::Theme {
                    name: new.clone(),
                    builtin: false,
                });
                self.push_resolved();
                self.rebuild_tree(Some(Node::Theme {
                    name: new,
                    builtin: false,
                }));
            }
            Err(e) => {
                self.theme_name_entry.set_text(&old);
                self.alert("Cannot rename theme", &e.to_string());
            }
        }
    }
    fn add_theme(&self) {
        let name = {
            let mut cfg = self.app_config.borrow_mut();
            let name = cfg.unique_theme_name("New Theme");
            let global = cfg.global_theme.clone();
            let _ = cfg.set_theme(&name, global);
            name
        };
        self.config_dirty.store(true, Ordering::Relaxed);
        self.rebuild_tree(Some(Node::Theme {
            name,
            builtin: false,
        }));
    }

    fn duplicate_theme(&self) {
        let current = self.current_theme.borrow().clone();
        let name = {
            let mut cfg = self.app_config.borrow_mut();
            let (base, theme) = match &current {
                Some(Node::GlobalTheme) => ("Global copy".to_string(), cfg.global_theme.clone()),
                Some(Node::Theme { name, .. }) => {
                    let theme = cfg
                        .theme_library()
                        .lookup_named(name)
                        .unwrap_or_else(|| cfg.global_theme.clone());
                    (name.clone(), theme)
                }
                _ => return,
            };
            let name = cfg.unique_theme_name(&base);
            let _ = cfg.set_theme(&name, theme);
            name
        };
        self.config_dirty.store(true, Ordering::Relaxed);
        self.rebuild_tree(Some(Node::Theme {
            name,
            builtin: false,
        }));
    }

    fn delete_theme(&self) {
        let Some(Node::Theme {
            name,
            builtin: false,
        }) = self.current_theme.borrow().clone()
        else {
            return;
        };
        self.pull_from_live();
        let result = self.app_config.borrow_mut().delete_theme(&name);
        match result {
            Ok(()) => {
                self.config_dirty.store(true, Ordering::Relaxed);
                self.rebuild_tree(Some(Node::ThemesRoot));
            }
            Err(e) => self.alert("Cannot delete theme", &e.to_string()),
        }
    }
    // ---- Group page actions ---------------------------------------------------

    fn group_name_changed(&self, text: String) {
        if self.loading.get() {
            return;
        }
        let Some(id) = self.current_group.borrow().clone() else {
            return;
        };
        if let Some(group) = self.app_config.borrow_mut().group_mut(&id) {
            group.name = text;
        }
        self.config_dirty.store(true, Ordering::Relaxed);
    }

    fn group_theme_selected(&self) {
        if self.loading.get() {
            return;
        }
        let Some(id) = self.current_group.borrow().clone() else {
            return;
        };
        let idx = self.group_theme_dropdown.selected() as usize;
        let theme = {
            let names = self.group_theme_names.borrow();
            match names.get(idx) {
                Some(_) if idx == 0 => ThemeRef::Global,
                Some(name) => ThemeRef::named(name),
                None => return,
            }
        };
        if let Some(group) = self.app_config.borrow_mut().group_mut(&id) {
            group.theme = theme;
        }
        self.push_resolved();
        self.rebuild_tree(Some(Node::Group { id }));
    }

    fn add_group(&self) {
        let id = self.app_config.borrow_mut().add_group("New Group");
        self.config_dirty.store(true, Ordering::Relaxed);
        self.rebuild_tree(Some(Node::Group { id }));
    }

    fn delete_group(&self) {
        let Some(id) = self.current_group.borrow().clone() else {
            return;
        };
        self.pull_from_live();
        self.app_config.borrow_mut().delete_group(&id);
        for panel in self.panels() {
            let mut guard = panel.blocking_write();
            if guard.group_id() == Some(id.as_str()) {
                guard.set_group_id(None);
            }
        }
        self.push_resolved();
        self.rebuild_tree(Some(Node::GroupsRoot));
    }
    // ---- Panel page actions ---------------------------------------------------

    fn panel_theme_selected(&self) {
        if self.loading.get() {
            return;
        }
        let Some(id) = self.current_panel.borrow().clone() else {
            return;
        };
        let idx = self.panel_theme_dropdown.selected() as usize;
        let choice = match self.panel_theme_choices.borrow().get(idx) {
            Some(c) => c.clone(),
            None => return,
        };
        self.pull_from_live();
        let theme = {
            let cfg = self.app_config.borrow();
            match choice {
                ThemeChoice::Inherit => ThemeRef::Inherit,
                ThemeChoice::Global => ThemeRef::Global,
                ThemeChoice::Named(name) => ThemeRef::named(name),
                // Start the private theme from what the panel resolves to now,
                // so switching to Inline does not change its look.
                ThemeChoice::Inline => ThemeRef::inline(cfg.resolve_theme_for_panel_id(&id)),
            }
        };
        self.assign_panel_theme(&id, theme);
    }

    fn panel_inline_edited(&self) {
        if self.loading.get() {
            return;
        }
        let Some(id) = self.current_panel.borrow().clone() else {
            return;
        };
        let theme = ThemeRef::inline(self.panel_inline_widget.get_config());
        let _ = self
            .app_config
            .borrow_mut()
            .set_panel_theme(&id, theme.clone());
        self.with_live_panel(&id, |p| p.set_theme_ref(theme));
        self.push_resolved();
    }
    // ---- Drag and drop --------------------------------------------------------

    fn handle_drop(self: &Rc<Self>, source: Node, target: Node) -> bool {
        match (source.drag_payload(), target) {
            (Some(DragPayload::Panel(pid)), Node::Group { id }) => {
                self.move_panel(&pid, Some(id));
                true
            }
            (Some(DragPayload::Panel(pid)), Node::Panel { id: other }) => {
                // Dropping onto a member puts the panel in that member's group
                let gid = self.panels().into_iter().find_map(|p| {
                    let guard = p.blocking_read();
                    (guard.id == other).then(|| guard.group_id().map(|s| s.to_string()))
                });
                match gid {
                    Some(gid) => {
                        self.move_panel(&pid, gid);
                        true
                    }
                    None => false,
                }
            }
            (Some(DragPayload::Panel(pid)), Node::UngroupedRoot) => {
                self.move_panel(&pid, None);
                true
            }
            (Some(DragPayload::Theme(theme)), Node::Group { id }) => {
                if let Some(group) = self.app_config.borrow_mut().group_mut(&id) {
                    group.theme = theme;
                }
                self.push_resolved();
                self.rebuild_tree(Some(Node::Group { id }));
                true
            }
            (Some(DragPayload::Theme(theme)), Node::Panel { id }) => {
                let is_inline = self
                    .live_theme_ref(&id)
                    .map(|t| t.is_inline())
                    .unwrap_or(false);
                if is_inline {
                    self.confirm_replace_inline(id, theme);
                } else {
                    self.assign_panel_theme(&id, theme);
                }
                true
            }
            _ => false,
        }
    }

    fn move_panel(&self, panel_id: &str, group_id: Option<String>) {
        self.pull_from_live();
        if let Err(e) = self
            .app_config
            .borrow_mut()
            .set_panel_group(panel_id, group_id.as_deref())
        {
            log::warn!("Cannot move panel {}: {}", panel_id, e);
            return;
        }
        self.with_live_panel(panel_id, |p| p.set_group_id(group_id));
        self.push_resolved();
        self.rebuild_tree(Some(Node::Panel {
            id: panel_id.to_string(),
        }));
    }

    fn assign_panel_theme(&self, panel_id: &str, theme: ThemeRef) {
        let _ = self
            .app_config
            .borrow_mut()
            .set_panel_theme(panel_id, theme.clone());
        self.with_live_panel(panel_id, |p| p.set_theme_ref(theme));
        self.push_resolved();
        self.rebuild_tree(Some(Node::Panel {
            id: panel_id.to_string(),
        }));
    }
    fn confirm_replace_inline(self: &Rc<Self>, panel_id: String, theme: ThemeRef) {
        let Some(window) = self.window() else {
            return;
        };
        let weak = Rc::downgrade(self);
        glib::MainContext::default().spawn_local(async move {
            let dialog = gtk4::AlertDialog::builder()
                .modal(true)
                .message("Replace this panel's inline theme?")
                .detail("The panel has a private theme. Assigning a library theme discards it.")
                .buttons(["Cancel", "Replace"])
                .cancel_button(0)
                .default_button(1)
                .build();
            if let Ok(1) = dialog.choose_future(Some(&window)).await {
                if let Some(state) = weak.upgrade() {
                    state.assign_panel_theme(&panel_id, theme);
                }
            }
        });
    }

    fn alert(&self, message: &str, detail: &str) {
        let dialog = gtk4::AlertDialog::builder()
            .modal(true)
            .message(message)
            .detail(detail)
            .buttons(["OK"])
            .build();
        dialog.show(self.window().as_ref());
    }
}

fn describe_group_theme(theme: &ThemeRef) -> String {
    match theme {
        ThemeRef::Inherit | ThemeRef::Global => "Global".to_string(),
        other => other.describe(),
    }
}
