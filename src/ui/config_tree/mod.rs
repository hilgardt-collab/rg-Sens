//! Tree-based Panel Properties dialog (docs/config-ui-redesign.md, Option A).
//!
//! A tree of the panel's structure on the left (geometry, theme, source,
//! display and its sections, combo content groups and items, background,
//! appearance), one editor page on the right, a breadcrumb on top. Edits
//! apply to the live panel after a short debounce; Revert restores the state
//! the window opened with. Editors are built on first selection and kept for
//! the window's lifetime.
//!
//! The existing `*_config_widget.rs` editors are reused through
//! `editor_registry`; the grid applies the assembled `PanelData` through
//! `GridLayout::apply_panel_data`.

pub mod combo_items;
pub mod field_editors;
mod node;
mod pages;
pub mod schema_combo;
pub mod schema_editor;
mod state;
mod util;

use crate::config::AppConfig;
use crate::core::Panel;
use crate::ui::GridLayout;
use gtk4::prelude::*;
use gtk4::{
    gio, glib, Box as GtkBox, Button, Label, ListView, Orientation, Paned, ScrolledWindow,
    SignalListItemFactory, SingleSelection, Stack, TreeExpander, TreeListModel, TreeListRow,
    Window,
};
use node::NodeKind;
use state::{start_polling, State};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use tokio::sync::RwLock;

thread_local! {
    /// One window per panel; the strong owner of its state (closures hold
    /// `Weak<State>`).
    static OPEN: RefCell<HashMap<String, (glib::WeakRef<Window>, Rc<State>)>> =
        RefCell::new(HashMap::new());
}

/// Close the properties dialog of one panel (the panel is being deleted).
pub fn close_tree_properties(panel_id: &str) {
    let window = OPEN.with(|o| o.borrow().get(panel_id).and_then(|(w, _)| w.upgrade()));
    if let Some(w) = window {
        w.close();
    }
}

/// Close every open properties dialog (the main window is closing).
pub fn close_all_tree_properties() {
    let windows: Vec<Window> = OPEN.with(|o| {
        o.borrow()
            .values()
            .filter_map(|(w, _)| w.upgrade())
            .collect()
    });
    for w in windows {
        w.close();
    }
}

/// Open (or present) the tree properties dialog for a panel.
///
/// `anchor` is any widget in the main window, used to find the parent window.
/// The grid is found through `grid_layout::grid_weak()`.
pub fn open_tree_properties(panel: &Arc<RwLock<Panel>>, anchor: &impl IsA<gtk4::Widget>) {
    let Some(grid) = super::grid_layout::grid_weak() else {
        log::warn!("Tree properties dialog: grid reference not set");
        return;
    };
    let Some(app_config) = grid.upgrade().map(|g| g.borrow().app_config()) else {
        return;
    };
    let parent = anchor.root().and_then(|r| r.downcast::<Window>().ok());
    show_tree_properties_dialog(parent.as_ref(), panel, grid, app_config);
}

pub fn show_tree_properties_dialog(
    parent: Option<&Window>,
    panel: &Arc<RwLock<Panel>>,
    grid: Weak<RefCell<GridLayout>>,
    app_config: Rc<RefCell<AppConfig>>,
) {
    let (panel_id, data) = {
        let guard = panel.blocking_read();
        (guard.id.clone(), guard.to_data())
    };

    if let Some(existing) = OPEN.with(|o| o.borrow().get(&panel_id).and_then(|(w, _)| w.upgrade()))
    {
        existing.present();
        return;
    }

    let title = format!(
        "Panel Properties — {}",
        util::displayer_display_name(data.displayer_config.displayer_type())
    );
    let window = Window::builder()
        .title(title)
        .modal(false)
        .default_width(960)
        .default_height(680)
        .build();
    if let Some(p) = parent {
        window.set_transient_for(Some(p));
    }

    // ---- Tree ---------------------------------------------------------------
    let tree_spec: Rc<RefCell<HashMap<String, Vec<NodeKind>>>> =
        Rc::new(RefCell::new(HashMap::new()));
    let root_store = gio::ListStore::new::<glib::BoxedAnyObject>();
    let tree_model = {
        let spec = tree_spec.clone();
        TreeListModel::new(root_store.clone(), false, true, move |obj| {
            let kind = NodeKind::from_object(obj)?;
            let kids = spec.borrow().get(&kind.id()).cloned()?;
            if kids.is_empty() {
                return None;
            }
            let store = gio::ListStore::new::<glib::BoxedAnyObject>();
            for k in kids {
                store.append(&k.boxed());
            }
            Some(store.upcast())
        })
    };
    let selection = SingleSelection::new(Some(tree_model));
    selection.set_autoselect(false);

    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_transition_type(gtk4::StackTransitionType::None);

    let breadcrumb = GtkBox::new(Orientation::Horizontal, 2);
    breadcrumb.set_margin_start(8);
    breadcrumb.set_margin_end(8);
    breadcrumb.set_margin_top(6);

    let state = Rc::new(State {
        panel: panel.clone(),
        panel_id: panel_id.clone(),
        grid,
        app_config,
        window: window.downgrade(),
        snapshot: RefCell::new(data.clone()),
        applied: RefCell::new(data.clone()),
        combo_source: RefCell::new(None),
        theme_ref: RefCell::new(data.theme.clone()),
        theme_baseline: RefCell::new(None),
        source_editor: RefCell::new(None),
        display_editor: RefCell::new(None),
        display_needs_poll: Cell::new(false),
        source_needs_poll: Cell::new(false),
        display_pages: RefCell::new(Vec::new()),
        display_subpages: RefCell::new(Vec::new()),
        theme_page: RefCell::new(None),
        background_editor: RefCell::new(None),
        inline_theme_widget: RefCell::new(None),
        item_editors: RefCell::new(HashMap::new()),
        item_theme_refreshers: Rc::new(RefCell::new(Vec::new())),
        item_cleanup: Rc::new(RefCell::new(Vec::new())),
        geometry: RefCell::new(None),
        appearance: RefCell::new(None),
        theme_controls: RefCell::new(None),
        tree_spec,
        filter: RefCell::new(String::new()),
        root_store: root_store.clone(),
        selection: selection.clone(),
        stack: stack.clone(),
        breadcrumb: breadcrumb.clone(),
        pages: RefCell::new(Default::default()),
        loading: Cell::new(false),
        commit_scheduled: Cell::new(false),
        poll_last: RefCell::new(HashMap::new()),
        row_labels: RefCell::new(HashMap::new()),
        alive: Rc::new(Cell::new(true)),
    });
    let weak = Rc::downgrade(&state);

    let factory = SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
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
        summary.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        summary.set_max_width_chars(28);
        hbox.append(&title);
        hbox.append(&summary);
        expander.set_child(Some(&hbox));
        item.set_child(Some(&expander));
    });
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
            let Some(kind) = row.item().and_then(|o| NodeKind::from_object(&o)) else {
                return;
            };
            let title = hbox.first_child().and_then(|c| c.downcast::<Label>().ok());
            let summary = hbox.last_child().and_then(|c| c.downcast::<Label>().ok());
            if let Some(state) = weak.upgrade() {
                let (t, s) = state.row_text(&kind);
                if let Some(l) = &title {
                    l.set_text(&t);
                }
                if let Some(l) = &summary {
                    l.set_text(&s);
                }
                if let (Some(t), Some(s)) = (title, summary) {
                    state
                        .row_labels
                        .borrow_mut()
                        .insert(kind.id(), (kind, t.downgrade(), s.downgrade()));
                }
            }
        });
    }
    {
        let weak = weak.clone();
        factory.connect_unbind(move |_, item| {
            let Some(item) = item.downcast_ref::<gtk4::ListItem>() else {
                return;
            };
            if let Some(kind) = item
                .item()
                .and_then(|o| o.downcast::<TreeListRow>().ok())
                .and_then(|row| row.item())
                .and_then(|o| NodeKind::from_object(&o))
            {
                if let Some(state) = weak.upgrade() {
                    state.row_labels.borrow_mut().remove(&kind.id());
                }
            }
            if let Some(expander) = item.child().and_then(|c| c.downcast::<TreeExpander>().ok()) {
                expander.set_list_row(None);
            }
        });
    }

    let list_view = ListView::new(Some(selection.clone()), Some(factory));
    list_view.set_vexpand(true);
    let tree_scroll = ScrolledWindow::new();
    tree_scroll.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
    tree_scroll.set_child(Some(&list_view));
    tree_scroll.set_vexpand(true);
    tree_scroll.set_margin_start(8);
    tree_scroll.set_margin_top(8);
    tree_scroll.set_margin_bottom(8);

    // Search box: filters the tree to matching nodes and their ancestors.
    let search = gtk4::SearchEntry::new();
    search.set_placeholder_text(Some("Search settings"));
    search.set_margin_start(8);
    search.set_margin_top(8);
    {
        let weak = weak.clone();
        search.connect_search_changed(move |entry| {
            if let Some(state) = weak.upgrade() {
                state.set_filter(&entry.text());
            }
        });
    }
    let tree_column = GtkBox::new(Orientation::Vertical, 0);
    tree_column.append(&search);
    tree_column.append(&tree_scroll);

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_start_child(Some(&tree_column));
    paned.set_end_child(Some(&stack));
    paned.set_position(300);
    paned.set_shrink_start_child(false);
    paned.set_shrink_end_child(false);
    paned.set_vexpand(true);

    let button_row = GtkBox::new(Orientation::Horizontal, 6);
    button_row.set_halign(gtk4::Align::End);
    button_row.set_margin_top(8);
    button_row.set_margin_bottom(8);
    button_row.set_margin_end(8);
    let revert_btn = Button::with_label("Revert");
    revert_btn.set_tooltip_text(Some("Restore the panel as it was when this window opened"));
    let close_btn = Button::with_label("Close");
    close_btn.add_css_class("suggested-action");
    button_row.append(&revert_btn);
    button_row.append(&close_btn);

    let vbox = GtkBox::new(Orientation::Vertical, 0);
    vbox.append(&breadcrumb);
    vbox.append(&paned);
    vbox.append(&button_row);
    window.set_child(Some(&vbox));

    // ---- Signals -------------------------------------------------------------
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
        revert_btn.connect_clicked(move |_| {
            if let Some(state) = weak.upgrade() {
                state.revert();
            }
        });
    }
    {
        let window_weak = window.downgrade();
        close_btn.connect_clicked(move |_| {
            if let Some(w) = window_weak.upgrade() {
                w.close();
            }
        });
    }
    {
        let panel_id = panel_id.clone();
        window.connect_close_request(move |w| {
            if let Some((_, state)) = OPEN.with(|o| o.borrow_mut().remove(&panel_id)) {
                state.alive.set(false);
                state.cleanup_editors();
            }
            w.set_child(None::<&gtk4::Widget>);
            glib::Propagation::Proceed
        });
    }

    OPEN.with(|o| {
        o.borrow_mut()
            .insert(panel_id, (window.downgrade(), state.clone()))
    });

    state.reset_from(data);
    start_polling(&state);
    window.present();
}

impl State {
    pub(super) fn show_selected(self: &Rc<Self>) {
        let Some(kind) = self.selected_node() else {
            return;
        };
        let id = self.ensure_page(&kind);
        self.stack.set_visible_child_name(&id);
        self.update_breadcrumb(&kind);
        self.reset_poll_baseline();
    }
}
