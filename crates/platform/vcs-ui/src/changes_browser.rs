//! The changes browser shared by the Push and Rollback dialogs.
use std::cell::Cell;
use std::collections::{BTreeMap, HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, ClickEvent, Context, EventEmitter, FocusHandle, Focusable, FontWeight,
    IntoElement, Pixels, Render, UniformListScrollHandle, Window, canvas, div, point, prelude::*,
    px, uniform_list,
};
use themes::UiRole;
use ui::components::{DISABLED_ALPHA, Tooltip, icon_button, tree_arrow};
use ui::form::MenuPopup;
use ui::icons::{Icon, IconName};
use ui::menu::MenuItem;
use ui::theme::ThemeExt;
use vcs::Change;

use crate::commit::tree::{Inclusion, file_count, origin_text, parent_text};
use crate::commit::tri_checkbox;
use crate::status_role;

ui::ui_commands! {
    COMMANDS, scope = "DialogChangesBrowser";
    ShowDiff => "dialog-changes.diff", "Show Diff", "Shows a diff of the selected change.";
    EditSource => "dialog-changes.edit", "Edit Source", "Opens the selected file in the editor.";
    ViewOptions => "dialog-changes.options", "View Options", "Shows grouping options for changed files.";
    GroupDirectory => "dialog-changes.group-directory", "Directory", "Toggles grouping changed files by directory.";
    ExpandAll => "dialog-changes.expand", "Expand All", "Expands all change groups.";
    CollapseAll => "dialog-changes.collapse", "Collapse All", "Collapses all change groups.";
    Up => "dialog-changes.up", "Up", "Selects the previous visible change row.";
    Down => "dialog-changes.down", "Down", "Selects the next visible change row.";
    Left => "dialog-changes.left", "Collapse or Go to Parent", "Collapses the group or selects its parent.";
    Right => "dialog-changes.right", "Expand or Go to Child", "Expands the group or selects its first child.";
    Toggle => "dialog-changes.toggle", "Include or Exclude", "Toggles the selected files for rollback.";
    ToggleDetails => "dialog-changes.details", "Show Commit Details", "Toggles details of the selected commit.";
}

pub enum BrowserEvent {
    ShowDiff(PathBuf),
    EditSource(PathBuf),
    ToggleDetails,
    InclusionChanged,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Node {
    Changes,
    Directory(PathBuf),
    File(usize),
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

struct TreeRow {
    node: Node,
    depth: usize,
    files: Range<usize>,
}

#[derive(Default)]
struct Folder {
    folders: BTreeMap<PathBuf, Folder>,
    files: Vec<usize>,
}

impl Folder {
    fn flatten(
        &self,
        node: Option<Node>,
        depth: usize,
        rows: &mut Vec<TreeRow>,
        order: &mut Vec<usize>,
    ) {
        let start = rows.len();
        let first = order.len();
        let child_depth = depth + usize::from(node.is_some());
        if let Some(node) = node {
            rows.push(TreeRow {
                node,
                depth,
                files: first..first,
            });
        }
        for (path, folder) in &self.folders {
            folder.flatten(
                Some(Node::Directory(path.clone())),
                child_depth,
                rows,
                order,
            );
        }
        for file in &self.files {
            let pos = order.len();
            order.push(*file);
            rows.push(TreeRow {
                node: Node::File(*file),
                depth: child_depth,
                files: pos..pos + 1,
            });
        }
        if child_depth > depth {
            rows[start].files.end = order.len();
        }
    }
}

pub struct ChangesBrowser {
    root: PathBuf,
    changelist: bool,
    file_icon: Rc<dyn Fn(&Path) -> IconName>,
    changes: Vec<Change>,
    included: HashSet<PathBuf>,
    grouped: bool,
    collapsed: HashSet<Node>,
    selected: Option<Node>,
    rows: Vec<TreeRow>,
    order: Vec<usize>,
    focus: FocusHandle,
    options_bounds: Rc<Cell<Bounds<Pixels>>>,
    menu: Option<MenuPopup>,
    scroll: UniformListScrollHandle,
    details: (bool, bool),
}

impl EventEmitter<BrowserEvent> for ChangesBrowser {}

impl Focusable for ChangesBrowser {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl ChangesBrowser {
    pub fn new(
        root: PathBuf,
        changelist: bool,
        file_icon: Rc<dyn Fn(&Path) -> IconName>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            root,
            changelist,
            file_icon,
            changes: Vec::new(),
            included: HashSet::new(),
            grouped: true,
            collapsed: HashSet::new(),
            selected: None,
            rows: Vec::new(),
            order: Vec::new(),
            focus: cx.focus_handle(),
            options_bounds: Rc::default(),
            menu: None,
            scroll: UniformListScrollHandle::new(),
            details: (false, false),
        }
    }

    pub fn set_changes(
        &mut self,
        changes: Vec<Change>,
        included: &[PathBuf],
        cx: &mut Context<Self>,
    ) {
        self.changes = changes;
        let requested: HashSet<_> = included.iter().collect();
        self.included = self
            .changes
            .iter()
            .filter(|c| requested.contains(&c.path))
            .map(|c| c.path.clone())
            .collect();
        self.selected = self
            .changes
            .iter()
            .position(|c| self.included.contains(&c.path))
            .map(Node::File);
        self.collapsed.clear();
        self.rebuild();
        cx.notify();
    }

    fn rebuild(&mut self) {
        let mut tree = Folder::default();
        let mut indices: Vec<_> = (0..self.changes.len()).collect();
        indices.sort_by_cached_key(|&i| {
            self.changes[i]
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
        });
        for i in indices {
            let path = &self.changes[i].path;
            let relative = path.strip_prefix(&self.root).unwrap_or(path);
            let mut folder = &mut tree;
            if self.grouped {
                let mut prefix = PathBuf::new();
                let parent = relative.parent().unwrap_or(Path::new(""));
                if parent.as_os_str().is_empty() {
                    folder = folder.folders.entry(prefix).or_default();
                } else {
                    for component in parent.components() {
                        prefix.push(component);
                        folder = folder.folders.entry(prefix.clone()).or_default();
                    }
                }
            }
            folder.files.push(i);
        }
        self.rows.clear();
        self.order.clear();
        if !self.changes.is_empty() {
            tree.flatten(
                self.changelist.then_some(Node::Changes),
                0,
                &mut self.rows,
                &mut self.order,
            );
        }
        if self
            .selected
            .as_ref()
            .is_some_and(|n| !self.rows.iter().any(|r| &r.node == n))
        {
            self.selected = None;
        }
    }

    pub fn set_commit_details(&mut self, enabled: bool, shown: bool, cx: &mut Context<Self>) {
        self.details = (enabled, shown);
        cx.notify();
    }

    pub fn changes(&self) -> &[Change] {
        &self.changes
    }
    pub fn is_included(&self, path: &Path) -> bool {
        self.included.contains(path)
    }
    pub fn included_changes(&self) -> Vec<Change> {
        self.changes
            .iter()
            .filter(|c| self.is_included(&c.path))
            .cloned()
            .collect()
    }

    pub fn toggle_file(&mut self, path: &Path, cx: &mut Context<Self>) {
        if let Some(ix) = self.changes.iter().position(|c| c.path == path)
            && let Some(row) = self.rows.iter().position(|r| r.node == Node::File(ix))
        {
            self.toggle(row, cx);
        }
    }

    fn toggle(&mut self, row: usize, cx: &mut Context<Self>) {
        let files = &self.order[self.rows[row].files.clone()];
        let include = files
            .iter()
            .any(|&i| !self.included.contains(&self.changes[i].path));
        for &i in files {
            let path = &self.changes[i].path;
            if include {
                self.included.insert(path.clone());
            } else {
                self.included.remove(path);
            }
        }
        cx.emit(BrowserEvent::InclusionChanged);
        cx.notify();
    }

    fn selected_file(&self) -> Option<PathBuf> {
        let row = self
            .selected
            .as_ref()
            .and_then(|node| self.rows.iter().find(|row| &row.node == node));
        let index = row.map(|r| r.files.start).unwrap_or_default();
        self.order.get(index).map(|&i| self.changes[i].path.clone())
    }

    fn show_diff(&mut self, _: &ShowDiff, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.selected_file() {
            cx.emit(BrowserEvent::ShowDiff(path));
        }
    }

    fn edit_source(&mut self, _: &EditSource, _: &mut Window, cx: &mut Context<Self>) {
        if !self.changelist
            && let Some(Node::File(i)) = self.selected
        {
            cx.emit(BrowserEvent::EditSource(self.changes[i].path.clone()));
        }
    }

    fn options(&mut self, _: &ViewOptions, window: &mut Window, cx: &mut Context<Self>) {
        let mut directory = MenuItem::action("Directory", GroupDirectory);
        if self.grouped {
            directory = directory.icon(IconName::CHECKED);
        }
        window.focus(&self.focus);
        if let Some(menu) = self.menu.take() {
            menu.close(cx);
        }
        let bounds = self.options_bounds.get();
        self.menu = MenuPopup::open(
            vec![MenuItem::header("Group By"), directory],
            None,
            point(bounds.left(), bounds.bottom()),
            &self.focus,
            window.is_window_active(),
            window,
            cx,
            |this: &mut Self, cx| {
                if let Some(menu) = this.menu.take() {
                    menu.close(cx);
                }
                cx.notify();
            },
        );
        cx.notify();
    }

    fn expand(&mut self, row: usize, cx: &mut Context<Self>) {
        let node = &self.rows[row].node;
        if !self.collapsed.remove(node) {
            self.collapsed.insert(node.clone());
        }
        cx.notify();
    }

    fn visible_rows(&self) -> Vec<usize> {
        let mut hidden_depth = None;
        let mut visible = Vec::new();
        for (i, row) in self.rows.iter().enumerate() {
            if hidden_depth.is_some_and(|d| row.depth > d) {
                continue;
            }
            visible.push(i);
            hidden_depth = self.collapsed.contains(&row.node).then_some(row.depth);
        }
        visible
    }

    fn selected_index(&self) -> Option<usize> {
        self.rows
            .iter()
            .position(|row| Some(&row.node) == self.selected.as_ref())
    }

    fn navigate(&mut self, direction: Direction, cx: &mut Context<Self>) {
        let visible = self.visible_rows();
        let Some(&first) = visible.first() else {
            return;
        };
        let pos = self
            .selected_index()
            .and_then(|i| visible.iter().position(|&v| v == i));
        let next = if let Some(pos) = pos {
            let index = visible[pos];
            let row = &self.rows[index];
            let group = !matches!(row.node, Node::File(_));
            match direction {
                Direction::Up => visible[pos.saturating_sub(1)],
                Direction::Down => visible[(pos + 1).min(visible.len() - 1)],
                Direction::Left if group && !self.collapsed.contains(&row.node) => {
                    self.expand(index, cx);
                    return;
                }
                Direction::Left => visible[..pos]
                    .iter()
                    .rev()
                    .copied()
                    .find(|&r| self.rows[r].depth < row.depth)
                    .unwrap_or(index),
                Direction::Right if group && self.collapsed.contains(&row.node) => {
                    self.expand(index, cx);
                    return;
                }
                Direction::Right if group => visible.get(pos + 1).copied().unwrap_or(index),
                Direction::Right => index,
            }
        } else {
            first
        };
        self.selected = Some(self.rows[next].node.clone());
        if let Some(index) = visible.iter().position(|&row| row == next) {
            self.scroll
                .scroll_to_item(index, gpui::ScrollStrategy::Center);
        }
        cx.notify();
    }

    pub fn state_json(&self) -> serde_json::Value {
        let visible = self.visible_rows();
        serde_json::json!({
            "grouped": self.grouped,
            "visible_count": visible.len(),
            "selected": self.selected_index().and_then(|i| visible.iter().position(|&row| row==i)),
        })
    }

    fn row(&self, i: usize, cx: &Context<Self>) -> AnyElement {
        let row = &self.rows[i];
        let theme = cx.theme();
        let muted = theme.color(UiRole::TextMuted);
        let selected = self.selected.as_ref() == Some(&row.node);
        let is_folder = !matches!(row.node, Node::File(_));
        let indent = 19. * row.depth as f32;
        let expanded = !self.collapsed.contains(&row.node);
        let mut line = div()
            .id(("change-browser-row", i))
            .relative()
            .h(px(24.))
            .flex()
            .items_center()
            .pl(px(12. + indent))
            .gap(px(6.))
            .whitespace_nowrap()
            .overflow_hidden()
            .when(selected, |d| d.bg(theme.color(UiRole::SelectionInactive)));
        for ancestor in 0..row.depth {
            line = line.child(
                div()
                    .absolute()
                    .left(px(19. + 19. * ancestor as f32))
                    .h_full()
                    .w(px(1.))
                    .bg(theme.color(UiRole::TreeIndentGuide)),
            );
        }
        line = line.child(
            div()
                .id(("change-chevron", i))
                .size(px(16.))
                .flex_none()
                .when(is_folder, |d| {
                    d.child(tree_arrow(expanded, cx)).on_click(cx.listener(
                        move |this, _: &ClickEvent, window, cx| {
                            cx.stop_propagation();
                            window.focus(&this.focus);
                            this.selected = Some(this.rows[i].node.clone());
                            this.expand(i, cx);
                        },
                    ))
                }),
        );
        if self.changelist {
            let files = &self.order[row.files.clone()];
            let checked = files
                .iter()
                .filter(|&&ix| self.included.contains(&self.changes[ix].path))
                .count();
            let inclusion = if checked == 0 {
                Inclusion::None
            } else if checked == files.len() {
                Inclusion::All
            } else {
                Inclusion::Some
            };
            line = line.child(
                div()
                    .id(("change-toggle", i))
                    .child(tri_checkbox(("change-box", i), inclusion, cx))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        window.focus(&this.focus);
                        this.selected = Some(this.rows[i].node.clone());
                        this.toggle(i, cx);
                    })),
            );
        }
        match &row.node {
            Node::Changes => {
                line = line.child(div().font_weight(FontWeight::SEMIBOLD).child("Changes"));
            }
            Node::Directory(path) => {
                line = line.child(Icon::new(IconName::FOLDER));
                if let Some(name) = path.file_name() {
                    line = line.child(name.to_string_lossy().into_owned());
                }
            }
            Node::File(ix) => {
                let change = &self.changes[*ix];
                let color = status_role(change.status)
                    .map_or(theme.color(UiRole::Text), |r| theme.color(r));
                line = line
                    .child(Icon::new((self.file_icon)(&change.path)))
                    .child(
                        div().text_color(color).child(
                            change
                                .path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned(),
                        ),
                    )
                    .children(
                        origin_text(change, &self.root)
                            .map(|label| div().text_color(muted).child(label)),
                    )
                    .when(!self.grouped, |d| {
                        d.child(
                            div()
                                .text_color(muted)
                                .child(parent_text(&change.path, &self.root)),
                        )
                    });
            }
        }
        line.when(is_folder, |d| {
            d.child(div().text_color(muted).child(file_count(row.files.len())))
        })
        .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
            window.focus(&this.focus);
            this.selected = Some(this.rows[i].node.clone());
            if e.click_count() == 2 {
                if is_folder {
                    this.expand(i, cx);
                } else {
                    this.show_diff(&ShowDiff, window, cx);
                }
            }
            cx.notify();
        }))
        .into_any_element()
    }

    fn tool(
        &self,
        label: (&'static str, &'static str),
        icon: IconName,
        action: impl gpui::Action,
        enabled: bool,
        active: bool,
        cx: &App,
    ) -> gpui::Stateful<gpui::Div> {
        let (id, title) = label;
        let focus = self.focus.clone();
        icon_button(id, icon, px(26.), px(16.), active, cx)
            .debug_selector(move || id.into())
            .tooltip(Tooltip::text(title, None))
            .when(!enabled, |d| d.opacity(DISABLED_ALPHA))
            .when(enabled, |d| {
                d.on_click(move |_, window, cx| {
                    window.focus(&focus);
                    window.dispatch_action(action.boxed_clone(), cx);
                })
            })
    }
}

impl Render for ChangesBrowser {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = self.visible_rows();
        let bounds = self.options_bounds.clone();
        let separator = || {
            div()
                .flex_none()
                .h(px(20.))
                .w(px(1.))
                .mx(px(3.))
                .bg(cx.theme().color(UiRole::Divider))
        };
        let toolbar = div()
            .flex()
            .items_center()
            .flex_none()
            .h(px(34.))
            .px(px(7.))
            .child(self.tool(
                ("dialog-diff", "Show Diff"),
                IconName::DIFF,
                ShowDiff,
                !self.changes.is_empty(),
                false,
                cx,
            ))
            .child(separator())
            .child(
                self.tool(
                    ("dialog-view-options", "View Options"),
                    IconName::SHOW,
                    ViewOptions,
                    true,
                    self.menu.is_some(),
                    cx,
                )
                .relative()
                .child(
                    div()
                        .absolute()
                        .left(px(6.))
                        .top(px(6.))
                        .child(Icon::new(IconName::DROPDOWN_CORNER).size(px(16.))),
                )
                .child(
                    canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {})
                        .absolute()
                        .size_full(),
                ),
            )
            .when(!self.changelist, |d| {
                d.child(self.tool(
                    ("dialog-edit", "Edit Source"),
                    IconName::EDIT,
                    EditSource,
                    matches!(self.selected, Some(Node::File(_))),
                    false,
                    cx,
                ))
            })
            .when(!self.changelist, |d| {
                d.child(separator()).child(self.tool(
                    ("dialog-commit-details", "Show Commit Details"),
                    IconName::PREVIEW,
                    ToggleDetails,
                    self.details.0,
                    self.details.1,
                    cx,
                ))
            })
            .child(div().flex_1())
            .child(self.tool(
                ("dialog-expand", "Expand All"),
                IconName::EXPAND_ALL,
                ExpandAll,
                !self.rows.is_empty(),
                false,
                cx,
            ))
            .child(self.tool(
                ("dialog-collapse", "Collapse All"),
                IconName::COLLAPSE_ALL,
                CollapseAll,
                !self.rows.is_empty(),
                false,
                cx,
            ));
        div()
            .size_full()
            .flex()
            .flex_col()
            .track_focus(&self.focus)
            .key_context("DialogChangesBrowser")
            .on_action(cx.listener(|this, _: &Up, _, cx| this.navigate(Direction::Up, cx)))
            .on_action(cx.listener(|this, _: &Down, _, cx| this.navigate(Direction::Down, cx)))
            .on_action(cx.listener(|this, _: &Left, _, cx| this.navigate(Direction::Left, cx)))
            .on_action(cx.listener(|this, _: &Right, _, cx| this.navigate(Direction::Right, cx)))
            .on_action(cx.listener(|this, _: &Toggle, _, cx| {
                if this.changelist
                    && let Some(i) = this.selected_index()
                {
                    this.toggle(i, cx);
                }
            }))
            .on_action(cx.listener(Self::show_diff))
            .on_action(cx.listener(Self::edit_source))
            .on_action(cx.listener(Self::options))
            .on_action(cx.listener(|this, _: &GroupDirectory, _, cx| {
                this.grouped = !this.grouped;
                this.rebuild();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ExpandAll, _, cx| {
                this.collapsed.clear();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CollapseAll, _, cx| {
                this.collapsed = this
                    .rows
                    .iter()
                    .filter(|r| !matches!(r.node, Node::File(_)))
                    .map(|r| r.node.clone())
                    .collect();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleDetails, _, cx| {
                if this.details.0 {
                    cx.emit(BrowserEvent::ToggleDetails);
                }
            }))
            .child(toolbar)
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .when(self.changelist, |d| d.border_1())
                    .when(!self.changelist, |d| d.border_t_1())
                    .border_color(cx.theme().color(UiRole::Border))
                    .pt(px(4.))
                    .child(
                        uniform_list(
                            "dialog-changes-tree",
                            visible.len(),
                            cx.processor(move |this, range: Range<usize>, _, cx| {
                                range.map(|j| this.row(visible[j], cx)).collect()
                            }),
                        )
                        .track_scroll(self.scroll.clone())
                        .size_full(),
                    ),
            )
    }
}
