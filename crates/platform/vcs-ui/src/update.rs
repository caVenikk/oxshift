//! The Update Project dialog (`UpdateOptionsDialog` with
//! `GitUpdateOptionsPanel`): "Update Project" with the update methods as
//! radio buttons (Merge incoming changes into the current branch, Rebase
//! the current branch on top of incoming changes), Don't show again at the
//! bottom left, Cancel and OK. With Don't show again the next Update
//! Project runs at once with the chosen method (Shift opens the dialog in
//! the reference; here `vcs.update-project` with `dialog: true`).

use gpui::{
    App, ClickEvent, Context, EventEmitter, FocusHandle, Focusable, FontWeight, IntoElement,
    Render, Window, div, prelude::*, px,
};
use serde_json::{Value as Json, json};
use themes::UiRole;
use ui::components::{checkbox, popup_shadow};
use ui::form::{BarButton, BottomBar, ButtonRole};
use ui::theme::ThemeExt;
use vcs::UpdateMethod;

ui::ui_commands! {
    COMMANDS, scope = "UpdateDialog";
    /// Open Git's documentation for updating a branch.
    UpdateHelp => "update-dialog.help", "Update Project Help",
        "Opens Git's guide to integrating incoming changes.";
    /// OK.
    ConfirmUpdate => "update-dialog.ok", "OK",
        "Updates the project with the method chosen in the Update Project dialog.";
    /// Request a separate confirmation before resetting the local branch.
    ResetToRemote => "update-dialog.reset-to-remote", "Reset to the Remote Branch",
        "Asks to reset the current branch to its tracked remote branch.";
    /// Cancel.
    CancelUpdate => "update-dialog.cancel", "Cancel", "Closes the Update Project dialog.";
    /// Choose the update method.
    SetMethod { method: String } => "update-dialog.set-method", "Update Method",
        "Chooses the update method of the Update Project dialog: merge or rebase.",
        r#"{"method": "rebase"}"#;
    /// Don't show again.
    ToggleDontShow => "update-dialog.toggle-dont-show", "Don't Show Again",
        "Toggles Don't show again of the Update Project dialog.";
}

/// Key context of the dialog.
pub const KEY_CONTEXT: &str = "UpdateDialog";
const WIDTH: f32 = 594.0;

/// How the dialog was closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateDialogEvent {
    Confirmed {
        method: UpdateMethod,
        dont_show: bool,
    },
    Cancelled,
    ResetToRemote,
}

pub struct UpdateDialog {
    focus: FocusHandle,
    ok_focus: FocusHandle,
    method: UpdateMethod,
    dont_show: bool,
}

impl EventEmitter<UpdateDialogEvent> for UpdateDialog {}

impl Focusable for UpdateDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.ok_focus.clone()
    }
}

impl UpdateDialog {
    pub fn new(method: UpdateMethod, cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            ok_focus: cx.focus_handle(),
            method,
            dont_show: false,
        }
    }

    pub fn title(&self) -> &'static str {
        "Update Project"
    }

    fn confirm(&mut self, _: &ConfirmUpdate, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(UpdateDialogEvent::Confirmed {
            method: self.method,
            dont_show: self.dont_show,
        });
    }

    fn cancel(&mut self, _: &CancelUpdate, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(UpdateDialogEvent::Cancelled);
    }

    fn set_method(&mut self, action: &SetMethod, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(m) = UpdateMethod::from_id(&action.method) {
            self.method = m;
            cx.notify();
        }
    }

    fn toggle_dont_show(&mut self, _: &ToggleDontShow, _: &mut Window, cx: &mut Context<Self>) {
        self.dont_show = !self.dont_show;
        cx.notify();
    }

    /// The state for `vcs.state`.
    pub fn state_json(&self) -> Json {
        let methods: Vec<&str> = [UpdateMethod::Merge, UpdateMethod::Rebase]
            .iter()
            .map(|m| m.description())
            .collect();
        json!({
            "title": self.title(),
            "methods": methods,
            "method": self.method.id(),
            "dont_show": self.dont_show,
        })
    }
}

impl Render for UpdateDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owned = cfg!(target_os = "macos");
        let size = window.viewport_size();
        let theme = cx.theme();
        let focus_bg = theme.color(UiRole::Hover);
        let methods = [UpdateMethod::Merge, UpdateMethod::Rebase]
            .into_iter()
            .map(|m| {
                let id = m.id();
                let selected = self.method == m;
                let radio = div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(px(16.0))
                    .rounded_full()
                    .border_1()
                    .bg(theme.color(UiRole::CheckboxBackground))
                    .border_color(theme.color(UiRole::CheckboxBorder))
                    .when(selected, |d| {
                        d.child(
                            div()
                                .rounded_full()
                                .size(px(6.0))
                                .bg(theme.color(UiRole::CheckboxCheck)),
                        )
                    });
                div()
                    .id(("update-method", m as usize))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(radio)
                    .child(m.description())
                    .when(owned, |row| {
                        row.focusable()
                            .tab_stop(true)
                            .focus(move |style| style.bg(focus_bg))
                    })
                    .debug_selector(move || format!("update-method-{id}"))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.set_method(
                            &SetMethod {
                                method: id.to_owned(),
                            },
                            window,
                            cx,
                        )
                    }))
            })
            .collect::<Vec<_>>();
        let dont_show = div()
            .id("update-dont-show")
            .when(owned, |row| {
                row.focusable()
                    .tab_stop(true)
                    .focus(move |style| style.bg(focus_bg))
            })
            .debug_selector(|| "update-dont-show".into())
            .flex()
            .flex_row()
            .items_center()
            .gap(px(8.0))
            .child(checkbox("update-dont-show-box", self.dont_show, cx))
            .child("Don't show again")
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.toggle_dont_show(&ToggleDontShow, window, cx)
            }));
        let bar = BottomBar::new()
            .separator(false)
            .status_padding(7.0)
            .help(ui::form::help_button(
                "update-dialog-help",
                Some(Box::new(UpdateHelp)),
                cx,
            ))
            .status(dont_show)
            .element(
                BarButton::new(
                    "update-dialog-reset",
                    "Reset to the Remote Branch",
                    ButtonRole::Other,
                    Box::new(ResetToRemote),
                )
                .render(cx),
                -20,
                26.0,
            )
            .button(
                BarButton::new(
                    "update-dialog-ok",
                    "OK",
                    ButtonRole::Ok,
                    Box::new(ConfirmUpdate),
                )
                .focus_handle(self.ok_focus.clone()),
            )
            .button(BarButton::new(
                "update-dialog-cancel",
                "Cancel",
                ButtonRole::Cancel,
                Box::new(CancelUpdate),
            ))
            .render(cx);
        let dialog =
            div()
                .id("update-dialog")
                .debug_selector(|| "update-dialog".into())
                .key_context(KEY_CONTEXT)
                .track_focus(&self.focus)
                .on_action(cx.listener(|_, _: &ResetToRemote, _, cx| {
                    cx.emit(UpdateDialogEvent::ResetToRemote)
                }))
                .on_action(cx.listener(|_, _: &UpdateHelp, _, cx| {
                    cx.open_url("https://git-scm.com/docs/git-pull")
                }))
                .on_action(cx.listener(Self::confirm))
                .on_action(cx.listener(Self::cancel))
                .on_action(cx.listener(Self::set_method))
                .on_action(cx.listener(Self::toggle_dont_show))
                .occlude()
                .flex()
                .flex_col()
                .when(owned, |d| d.size_full())
                .when(!owned, |d| {
                    d.w(px(WIDTH))
                        .rounded(px(10.0))
                        .border_1()
                        .border_color(theme.color(UiRole::PopupBorder))
                        .shadow(popup_shadow())
                })
                .bg(theme.color(UiRole::SurfaceRaised))
                .font_family(theme.ui_family())
                .text_size(theme.ui_size())
                .text_color(theme.color(UiRole::Text))
                .overflow_hidden()
                .when(!owned, |d| {
                    d.child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .h(px(40.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.title()),
                    )
                })
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .when(owned, |d| d.flex_1())
                        .gap(px(8.0))
                        .px(px(20.0))
                        .pt(px(19.0))
                        .pb(px(12.0))
                        .children(methods),
                )
                .child(bar);
        if owned {
            return dialog.into_any_element();
        }
        div()
            .id("update-dialog-backdrop")
            .occlude()
            .absolute()
            .top_0()
            .left_0()
            .w(size.width)
            .h(size.height)
            .flex()
            .items_center()
            .justify_center()
            .child(dialog)
            .into_any_element()
    }
}
