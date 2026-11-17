use std::path::PathBuf;

use editor::{Editor, EditorConfig};
use gpui::{App, AppContext, Entity, Window};
use text::Buffer;

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
    let name = path.to_string_lossy().into_owned();
    let side = |label: &str| SideTitle {
        label: label.to_owned().into(),
        path: Some(name.clone().into()),
        read_only: true,
        ..Default::default()
    };
    let request = DiffRequest {
        path: path.clone(),
        title: name.clone().into(),
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
        let mut buffer = Buffer::new(after.as_deref().unwrap_or_default());
        buffer.set_read_only(true);
        let mut editor = Editor::new(buffer, window, cx);
        editor.set_viewer(true);
        editor.set_path(Some(path));
        editor
    });
    cx.new(|cx| {
        let mut view = DiffView::new(right, window, cx);
        view.set_request(
            request,
            before.map(Into::into),
            None,
            EditorConfig::default(),
            ScrollTo::FirstChange,
            cx,
        );
        view
    })
}
