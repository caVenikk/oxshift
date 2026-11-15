use gpui::{
    AnyView, App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString, Size, Styled, Window,
    WindowHandle, div, px,
};
use ui::dialog_window::{self, DialogOptions};
use ui::theme::ThemeExt as _;

pub(super) enum GitDialog {
    Overlay(Option<FocusHandle>),
    Owned(WindowHandle<GitWindow>),
}

impl GitDialog {
    /// The three Git forms share the parent's focus and close lifecycle.
    pub(super) fn open<V, E>(
        dialog: &Entity<V>,
        title: impl Into<SharedString>,
        dimensions: (f32, f32),
        cancelled: E,
        parent: &mut Window,
        cx: &mut App,
    ) -> Option<Self>
    where
        V: Render + Focusable + EventEmitter<E>,
        E: Clone + 'static,
    {
        let focus = dialog.focus_handle(cx);
        if !cfg!(target_os = "macos") {
            let previous = parent.focused(cx);
            parent.focus(&focus);
            return Some(Self::Overlay(previous));
        }
        if dialog_window::modal_of(parent.window_handle(), cx).is_some() {
            return None;
        }
        let title = title.into();
        let view = dialog.clone();
        let size = Size {
            width: px(dimensions.0),
            height: px(dimensions.1),
        };
        let options = DialogOptions {
            title: title.clone(),
            size,
            min_size: Some(size),
            themed_title: true,
            activate: !crate::env::background(),
            ..Default::default()
        };
        dialog_window::open(parent, options, cx, move |window, cx| {
            window.focus(&focus);
            let weak = view.downgrade();
            window.on_window_should_close(cx, move |window, cx| {
                if dialog_window::modal_of(window.window_handle(), cx).is_none() {
                    let _ = weak.update(cx, |_, cx| cx.emit(cancelled.clone()));
                }
                false
            });
            cx.new(|_| GitWindow {
                title,
                view: view.into(),
            })
        })
        .map(Self::Owned)
    }

    pub(super) fn close(&self, parent: &mut Window, cx: &mut App) {
        match self {
            Self::Owned(window) => dialog_window::close((*window).into(), cx),
            Self::Overlay(Some(focus)) => parent.focus(focus),
            Self::Overlay(None) => {}
        }
    }
}

pub(super) struct GitWindow {
    title: SharedString,
    view: AnyView,
}

impl Render for GitWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .on_key_down(|event, window, cx| {
                let key = &event.keystroke;
                if key.key == "tab"
                    && !(key.modifiers.alt
                        || key.modifiers.control
                        || key.modifiers.platform
                        || key.modifiers.function)
                {
                    if key.modifiers.shift {
                        window.focus_prev();
                    } else {
                        window.focus_next();
                    }
                    cx.stop_propagation();
                }
            })
            .capture_key_up(|event, window, _| {
                if event.keystroke.key == "enter" && !event.keystroke.modifiers.modified() {
                    window.prevent_default();
                }
            })
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().color(themes::UiRole::SurfaceRaised))
            .children(dialog_window::title_bar(self.title.clone(), true, cx))
            .child(div().flex_1().min_h(px(0.0)).child(self.view.clone()))
            .children(dialog_window::blocker(window, cx))
            .child(ui::pointer::frame())
    }
}
