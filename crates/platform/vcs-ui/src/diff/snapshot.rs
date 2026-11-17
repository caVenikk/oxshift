use std::path::PathBuf;

use editor::{Editor, EditorConfig};
use gpui::{
    AnyWindowHandle, App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, Render,
    SharedString, Window, WindowHandle, div, prelude::*, px, size,
};
use text::Buffer;
use themes::UiRole;
use ui::dialog_window::{self, DialogOptions};
use ui::theme::ThemeExt;

use super::{DiffRequest, DiffView, ScrollTo, Side, SideTitle};

/// Compare two immutable snapshots without opening a worktree document.
pub fn snapshot_view(
    path: PathBuf,
    before: Option<String>,
    after: Option<String>,
    labels: (&str, &str),
    window: &mut Window,
    cx: &mut App,
) -> Entity<DiffView> {
    let title = path.to_string_lossy().into_owned().into();
    snapshot(
        title,
        Some(path),
        (before.map(Into::into), after.as_deref()),
        labels,
        window,
        cx,
    )
}

/// Compare two text snapshots that have no backing file.
pub fn text_snapshot_view(
    title: SharedString,
    before: &str,
    after: &str,
    labels: (&str, &str),
    window: &mut Window,
    cx: &mut App,
) -> Entity<DiffView> {
    snapshot(
        title,
        None,
        (Some(before.into()), Some(after)),
        labels,
        window,
        cx,
    )
}

fn snapshot(
    title: SharedString,
    path: Option<PathBuf>,
    (before, after): (Option<std::sync::Arc<str>>, Option<&str>),
    labels: (&str, &str),
    window: &mut Window,
    cx: &mut App,
) -> Entity<DiffView> {
    let side = |label: &str| SideTitle {
        label: label.to_owned().into(),
        path: path.as_ref().map(|_| title.clone()),
        read_only: true,
        ..Default::default()
    };
    let request = DiffRequest {
        path: path.clone().unwrap_or_default(),
        title: title.clone(),
        left: side(labels.0),
        right: side(labels.1),
        included: None,
        excluded: Vec::new(),
        one_side: if before.is_none() {
            Some(Side::Right)
        } else if after.is_none() {
            Some(Side::Left)
        } else {
            None
        },
        files: (0, 1),
        steps: (false, false),
        preview: false,
    };
    let right = cx.new(|cx| {
        let mut buffer = Buffer::new(after.unwrap_or_default());
        buffer.set_read_only(true);
        let mut editor = Editor::new(buffer, window, cx);
        editor.set_viewer(true);
        editor.set_path(path);
        editor
    });
    cx.new(|cx| {
        let mut view = DiffView::new(right, window, cx);
        view.set_request(
            request,
            before,
            None,
            EditorConfig::default(),
            ScrollTo::FirstChange,
            cx,
        );
        view
    })
}

/// A modal comparison owned by the window that supplied its file list.
#[derive(Clone, Copy)]
pub struct SnapshotWindow(WindowHandle<SnapshotHost>);

impl SnapshotWindow {
    pub fn open(title: SharedString, parent: &mut Window, cx: &mut App) -> Option<Self> {
        if dialog_window::modal_of(parent.window_handle(), cx).is_some() {
            return None;
        }
        let preferred = size(px(1000.0), px(700.0));
        let display = parent.display(cx).map(|display| display.bounds());
        let size = display.map_or(preferred, |bounds| {
            let screen = bounds.size;
            gpui::size(
                preferred.width.min(screen.width),
                preferred
                    .height
                    .min((screen.height - px(64.0)).max(px(1.0))),
            )
        });
        let owner = parent.bounds();
        let centered = owner.center() - gpui::point(size.width / 2.0, size.height / 2.0);
        let origin = display.map_or(centered, |screen| {
            let gap = px(16.0);
            let right = owner.right() + gap;
            let left = owner.left() - gap - size.width;
            let max_x = (screen.right() - size.width).max(screen.left());
            let max_y = (screen.bottom() - size.height - gap).max(screen.top());
            let (x, y) = if right <= max_x {
                (right, centered.y)
            } else if left >= screen.left() {
                (left, centered.y)
            } else {
                // Leave the owner's title and as much of its file tree visible
                // as the screen permits, without moving the user's window.
                let x = if owner.center().x <= screen.center().x {
                    max_x
                } else {
                    screen.left()
                };
                (x, owner.top() + px(32.0))
            };
            gpui::point(x, y.min(max_y).max(screen.top() + px(24.0)))
        });
        let options = DialogOptions {
            title: title.clone(),
            size,
            min_size: Some(gpui::size(
                px(360.0).min(size.width),
                px(240.0).min(size.height),
            )),
            themed_title: true,
            activate: parent.is_window_active(),
            ..Default::default()
        };
        dialog_window::open_at(parent, options, origin, cx, move |window, cx| {
            let focus = cx.focus_handle();
            window.focus(&focus);
            cx.new(|_| SnapshotHost {
                title,
                focus,
                content: SnapshotContent::Loading,
            })
        })
        .map(Self)
    }

    pub fn handle(&self) -> AnyWindowHandle {
        self.0.into()
    }

    pub fn is_open(&self, cx: &App) -> bool {
        cx.windows().contains(&self.handle())
    }

    pub fn close(&self, cx: &mut App) {
        dialog_window::close(self.handle(), cx);
    }

    pub fn set_view(&self, view: Entity<DiffView>, cx: &mut App) {
        let _ = self.0.update(cx, |host, window, cx| {
            window.focus(&view.focus_handle(cx));
            host.content = SnapshotContent::View(view);
            cx.notify();
        });
    }

    pub fn set_error(&self, error: String, cx: &mut App) {
        let _ = self.0.update(cx, |host, _, cx| {
            host.content = SnapshotContent::Error(error.into());
            cx.notify();
        });
    }
}

enum SnapshotContent {
    Loading,
    View(Entity<DiffView>),
    Error(SharedString),
}

struct SnapshotHost {
    title: SharedString,
    focus: FocusHandle,
    content: SnapshotContent,
}

impl Render for SnapshotHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match &self.content {
            SnapshotContent::View(view) => view.clone().into_any_element(),
            SnapshotContent::Loading => div().p(px(12.0)).child("Loading…").into_any_element(),
            SnapshotContent::Error(error) => {
                div().p(px(12.0)).child(error.clone()).into_any_element()
            }
        };
        let theme = cx.theme();
        div()
            .key_context("SnapshotDiffWindow")
            .track_focus(&self.focus)
            .on_action(|_: &super::CloseWindow, window, cx| {
                let handle = window.window_handle();
                cx.defer(move |cx| dialog_window::close(handle, cx));
            })
            .size_full()
            .flex()
            .flex_col()
            .font_family(theme.ui_family())
            .text_size(theme.ui_size())
            .text_color(theme.color(UiRole::Text))
            .bg(theme.color(UiRole::SurfaceRaised))
            .children(dialog_window::compact_title_bar(self.title.clone(), cx))
            .child(div().flex_1().min_h(px(0.0)).child(body))
            .children(dialog_window::blocker(window, cx))
            .child(ui::pointer::frame())
    }
}
