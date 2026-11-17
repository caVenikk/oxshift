//! The dialogs of the Commit tool window, over the main window:
//!
//! - "Rollback Changes" (`RollbackChangesDialog`): the changes with check
//!   boxes (the ones chosen ticked), the legend, "Delete local copies of
//!   added files" (enabled when an added file is ticked), Rollback and
//!   Close.
//! - "Git User Name Is Not Defined" (`GitUserNameNotDefinedDialog`):
//!   Name, E-mail, "Set properties globally", Cancel and "Set and Commit".

use std::path::{Path, PathBuf};
use std::rc::Rc;

use editor::Editor;
use gpui::{
    AnyElement, App, ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable, FontWeight,
    IntoElement, Render, SharedString, Window, div, prelude::*, px,
};
use serde_json::{Value as Json, json};
use themes::UiRole;
use ui::components::popup_shadow;
use ui::form::{BarButton, BottomBar, ButtonRole};
use ui::icons::IconName;
use ui::theme::ThemeExt;
use vcs::{Change, FileStatus};

use super::legend::Legend;
use crate::changes_browser::{BrowserEvent, ChangesBrowser};
use crate::diff::DiffView;
use crate::status_colors::status_role;

ui::ui_commands! {
    ROLLBACK_COMMANDS, scope = "RollbackDialog";
    /// Rollback.
    ConfirmRollback => "rollback-dialog.confirm", "Rollback",
        "Rolls back the ticked changes of the Rollback Changes dialog.";
    /// Close.
    CancelRollback => "rollback-dialog.cancel", "Close",
        "Closes the Rollback Changes dialog without a rollback.";
    /// Delete local copies of added files.
    ToggleDeleteAdded => "rollback-dialog.toggle-delete", "Delete Local Copies of Added Files",
        "Toggles \"Delete local copies of added files\" of the Rollback Changes dialog (when an added file is ticked).";
    /// A file's check box.
    ToggleRollbackFile { path: String } => "rollback-dialog.toggle", "Include File",
        "Ticks or unticks a change of the Rollback Changes dialog (relative to the repository root).",
        r#"{"path": "main.py"}"#;
}

ui::ui_commands! {
    USER_NAME_COMMANDS, scope = "UserNameDialog";
    /// Set and Commit.
    ConfirmUserName => "user-name-dialog.confirm", "Set and Commit",
        "Sets the user name and e-mail of the Git User Name Is Not Defined dialog and commits.";
    /// Cancel.
    CancelUserName => "user-name-dialog.cancel", "Cancel",
        "Closes the Git User Name Is Not Defined dialog; nothing is committed.";
    /// Tab.
    NextUserNameField => "user-name-dialog.next-field", "Next Field",
        "Moves the focus between Name and E-mail of the Git User Name Is Not Defined dialog.";
    /// Fill in the dialog.
    SetUserName { name: Option<String>, email: Option<String>, global: Option<bool> } => "user-name-dialog.set", "Set Name and E-mail",
        "Fills in Name, E-mail and Set properties globally of the Git User Name Is Not Defined dialog.",
        r#"{"name": "Name", "email": "name@example.com", "global": false}"#;
}

/// Key contexts.
pub const ROLLBACK_KEY_CONTEXT: &str = "RollbackDialog";
pub const USER_NAME_KEY_CONTEXT: &str = "UserNameDialog";

/// `changes.action.rollback.custom.title` with the operation's name.
pub const ROLLBACK_TITLE: &str = "Rollback Changes";
/// `changes.checkbox.delete.locally.added.files`.
pub const DELETE_ADDED: &str = "Delete local copies of added files";
/// `title.user.name.email.not.specified`.
pub const USER_NAME_TITLE: &str = "Git User Name Is Not Defined";
/// `validation.warning.set.name.email.for.git`,
/// `validation.error.email.no.at`.
pub const NAME_EMAIL_MISSING: &str = "You need to specify username and email for Git";
pub const EMAIL_NO_AT: &str = "The email address must contain the '@' symbol";

const ROLLBACK_WIDTH: f32 = 380.0;
const USER_NAME_WIDTH: f32 = 440.0;
const LABEL_WIDTH: f32 = 64.0;

/// The frame of a dialog over the main window: centred, with a backdrop
/// that takes the mouse.
fn frame(id: &'static str, dialog: impl IntoElement, window: &Window) -> AnyElement {
    let size = window.viewport_size();
    div()
        .id(id)
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

/// The box of a dialog with its title.
fn panel(
    id: &'static str,
    title: &str,
    width: f32,
    owned: bool,
    cx: &App,
) -> gpui::Stateful<gpui::Div> {
    let theme = cx.theme();
    div()
        .id(id)
        .debug_selector(move || id.into())
        .occlude()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .when(owned, |d| d.size_full())
        .when(!owned, |d| {
            d.w(px(width))
                .pt(px(16.0))
                .rounded(px(10.0))
                .border_1()
                .border_color(theme.color(UiRole::PopupBorder))
                .shadow(popup_shadow())
        })
        .bg(theme.color(UiRole::SurfaceRaised))
        .font_family(theme.ui_family())
        .text_size(theme.ui_size())
        .text_color(theme.color(UiRole::Text))
        .when(!owned, |d| {
            d.child(
                div()
                    .px(px(20.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(SharedString::from(title.to_owned())),
            )
        })
}

// ---- Rollback Changes ----------------------------------------------------------

/// How the Rollback Changes dialog was closed.
#[derive(Debug, Clone, PartialEq)]
pub enum RollbackEvent {
    Confirmed {
        changes: Vec<Change>,
        delete_added: bool,
    },
    Cancelled,
    ShowDiff(Change),
}

pub struct RollbackDialog {
    focus: FocusHandle,
    root: PathBuf,
    browser: Entity<ChangesBrowser>,
    delete_added: bool,
    preview: Option<RollbackPreview>,
    _browser_subscription: gpui::Subscription,
}

enum RollbackPreview {
    Loading(PathBuf),
    Ready(PathBuf, Option<String>, Option<String>),
    Error(String),
    View(Entity<DiffView>),
}

impl EventEmitter<RollbackEvent> for RollbackDialog {}

impl Focusable for RollbackDialog {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        if self.preview.is_none() && !self.browser.read(cx).changes().is_empty() {
            self.browser.focus_handle(cx)
        } else {
            self.focus.clone()
        }
    }
}

impl RollbackDialog {
    /// The dialog for the repository's `changes`, `included` ticked;
    /// `delete_added` is the check box's last state.
    pub fn new(
        root: &Path,
        changes: Vec<Change>,
        included: &[PathBuf],
        delete_added: bool,
        file_icon: Rc<dyn Fn(&Path) -> IconName>,
        cx: &mut Context<Self>,
    ) -> Self {
        let browser = cx.new(|cx| {
            let mut browser = ChangesBrowser::new(root.to_owned(), true, file_icon, cx);
            browser.set_changes(changes, included, cx);
            browser
        });
        let subscription = cx.subscribe(&browser, |this, browser, event, cx| {
            if let BrowserEvent::ShowDiff(path) = event
                && let Some(change) = browser.read(cx).changes().iter().find(|c| c.path == *path)
            {
                let change = change.clone();
                this.preview = Some(RollbackPreview::Loading(path.clone()));
                cx.emit(RollbackEvent::ShowDiff(change));
            }
            cx.notify();
        });
        Self {
            focus: cx.focus_handle(),
            root: root.to_owned(),
            browser,
            delete_added,
            preview: None,
            _browser_subscription: subscription,
        }
    }

    pub fn diff_loaded(
        &mut self,
        path: PathBuf,
        result: Result<(Option<String>, Option<String>), String>,
        cx: &mut Context<Self>,
    ) {
        if matches!(&self.preview, Some(RollbackPreview::Loading(requested)) if *requested == path)
        {
            self.preview = Some(match result {
                Ok((before, after)) => RollbackPreview::Ready(path, before, after),
                Err(error) => RollbackPreview::Error(error),
            });
            cx.notify();
        }
    }

    fn has_added(&self, cx: &App) -> bool {
        let browser = self.browser.read(cx);
        browser
            .changes()
            .iter()
            .any(|c| c.status == FileStatus::Added && browser.is_included(&c.path))
    }

    pub fn delete_added(&self) -> bool {
        self.delete_added
    }

    fn confirm(&mut self, _: &ConfirmRollback, _: &mut Window, cx: &mut Context<Self>) {
        let changes = self.browser.read(cx).included_changes();
        if changes.is_empty() || self.preview.is_some() {
            return;
        }
        let delete_added = self.delete_added && self.has_added(cx);
        cx.emit(RollbackEvent::Confirmed {
            changes,
            delete_added,
        });
    }

    fn cancel(&mut self, _: &CancelRollback, window: &mut Window, cx: &mut Context<Self>) {
        if self.preview.take().is_some() {
            window.focus(&self.browser.focus_handle(cx));
            cx.notify();
        } else {
            cx.emit(RollbackEvent::Cancelled);
        }
    }

    fn toggle_delete(&mut self, _: &ToggleDeleteAdded, _: &mut Window, cx: &mut Context<Self>) {
        if self.has_added(cx) {
            self.delete_added = !self.delete_added;
            cx.notify();
        }
    }

    fn toggle_file(&mut self, action: &ToggleRollbackFile, _: &mut Window, cx: &mut Context<Self>) {
        let mut path = self.root.clone();
        for part in action.path.split('/').filter(|p| !p.is_empty()) {
            path.push(part);
        }
        self.browser.update(cx, |b, cx| b.toggle_file(&path, cx));
    }

    pub fn state_json(&self, cx: &App) -> Json {
        let rel = |p: &Path| {
            p.strip_prefix(&self.root)
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| p.display().to_string())
        };
        let included = self.browser.read(cx).included_changes();
        let legend = Legend::of(&included, 0);
        json!({
            "kind": "rollback",
            "browser": self.browser.read(cx).state_json(),
            "preview": self.preview.is_some(),
            "title": ROLLBACK_TITLE,
            "files": self.browser.read(cx).changes().iter().map(|c| json!({
                "path": rel(&c.path),
                "status": c.status.id(),
                "included": self.browser.read(cx).is_included(&c.path),
            })).collect::<Vec<_>>(),
            "legend": legend.text(false),
            "delete_added": self.delete_added,
            "delete_added_enabled": self.has_added(cx),
            "ok": "Rollback",
            "ok_enabled": !included.is_empty(),
        })
    }
}

impl Render for RollbackDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if matches!(self.preview, Some(RollbackPreview::Ready(..)))
            && let Some(RollbackPreview::Ready(path, before, after)) = self.preview.take()
        {
            self.preview = Some(RollbackPreview::View(crate::diff::snapshot_view(
                path,
                before,
                after,
                ("HEAD", "Local"),
                window,
                cx,
            )));
        }
        let owned = cfg!(target_os = "macos");
        let dialog = panel("rollback-dialog", ROLLBACK_TITLE, ROLLBACK_WIDTH, owned, cx)
            .gap(px(0.))
            .when(!owned, |d| d.h(px(552.)))
            .key_context(ROLLBACK_KEY_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::toggle_delete))
            .on_action(cx.listener(Self::toggle_file));
        if let Some(preview) = &self.preview {
            if self.browser.focus_handle(cx).is_focused(window) || window.focused(cx).is_none() {
                window.focus(&self.focus);
            }
            let view = match preview {
                RollbackPreview::View(view) => view.clone().into_any_element(),
                RollbackPreview::Error(error) => {
                    div().p(px(16.)).child(error.clone()).into_any_element()
                }
                _ => div().p(px(16.)).child("Loading diff…").into_any_element(),
            };
            let back = BottomBar::new()
                .separator(false)
                .button(BarButton::new(
                    "rollback-diff-back",
                    "Back",
                    ButtonRole::Cancel,
                    Box::new(CancelRollback),
                ))
                .render(cx);
            let dialog = dialog
                .child(div().flex_1().min_h(px(0.)).child(view))
                .child(back);
            return if owned {
                dialog.into_any_element()
            } else {
                frame("rollback-dialog-backdrop", dialog, window)
            };
        }
        let theme = cx.theme();
        let included = self.browser.read(cx).included_changes();
        let legend = Legend::of(&included, 0);
        let legend_el =
            div()
                .flex()
                .flex_row()
                .children(
                    legend
                        .chunks(false)
                        .into_iter()
                        .enumerate()
                        .map(|(i, chunk)| {
                            let color = status_role(chunk.status)
                                .map_or(theme.color(UiRole::Text), |r| theme.color(r));
                            div().text_color(color).child(if i > 0 {
                                format!("{}{}", super::legend::SEPARATOR, chunk.text)
                            } else {
                                chunk.text
                            })
                        }),
                );
        let has_added = self.has_added(cx);
        let delete = if has_added {
            ui::form::check_row(
                "rollback-delete-added",
                "rollback-delete-added-check",
                self.delete_added,
                DELETE_ADDED,
                cx,
            )
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.toggle_delete(&ToggleDeleteAdded, window, cx)
            }))
        } else {
            div()
                .id("rollback-delete-added")
                .flex()
                .items_center()
                .text_color(theme.color(UiRole::TextDisabled))
                .child(ui::components::checkbox_with(
                    "rollback-delete-added-check",
                    self.delete_added,
                    false,
                    cx,
                ))
                .child(DELETE_ADDED)
        }
        .gap(px(8.))
        .debug_selector(|| "rollback-delete-added".into());
        let buttons = BottomBar::new()
            .separator(false)
            .button(
                BarButton::new(
                    "rollback-dialog-ok",
                    "Rollback",
                    ButtonRole::Ok,
                    Box::new(ConfirmRollback),
                )
                .enabled(!included.is_empty()),
            )
            .button(BarButton::new(
                "rollback-dialog-cancel",
                "Close",
                ButtonRole::Cancel,
                Box::new(CancelRollback),
            ))
            .render(cx);
        let dialog = dialog
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .ml(px(16.))
                    .mr(px(12.))
                    .pt(px(8.))
                    .child(self.browser.clone()),
            )
            .when(!included.is_empty(), |d| {
                d.child(div().px(px(16.)).pt(px(4.)).child(legend_el))
            })
            .child(
                div()
                    .flex_none()
                    .ml(px(16.))
                    .pt(px(5.))
                    .pb(px(4.))
                    .child(delete),
            )
            .child(buttons);
        if owned {
            dialog.into_any_element()
        } else {
            frame("rollback-dialog-backdrop", dialog, window)
        }
    }
}

// ---- Git User Name Is Not Defined ------------------------------------------------

/// How the user name dialog was closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserNameEvent {
    Confirmed {
        name: String,
        email: String,
        global: bool,
    },
    Cancelled,
}

pub struct UserNameDialog {
    name: Entity<Editor>,
    email: Entity<Editor>,
    global: bool,
    error: Option<&'static str>,
    /// The commit's name ("Commit", "Amend Commit") for "Set and …".
    action: &'static str,
}

impl EventEmitter<UserNameEvent> for UserNameDialog {}

impl Focusable for UserNameDialog {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.name.focus_handle(cx)
    }
}

impl UserNameDialog {
    /// The dialog with the proposed name and e-mail.
    pub fn new(
        name: &str,
        email: &str,
        global: bool,
        action: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = editor::field::new(name, None, true, window, cx);
        let email = editor::field::new(email, None, true, window, cx);
        Self {
            name,
            email,
            global,
            error: None,
            action,
        }
    }

    fn ok_label(&self) -> String {
        format!("Set and {}", self.action)
    }

    fn confirm(&mut self, _: &ConfirmUserName, _: &mut Window, cx: &mut Context<Self>) {
        let name = self.name.read(cx).text().trim().to_owned();
        let email = self.email.read(cx).text().trim().to_owned();
        self.error = if name.is_empty() || email.is_empty() {
            Some(NAME_EMAIL_MISSING)
        } else if !email.contains('@') {
            Some(EMAIL_NO_AT)
        } else {
            None
        };
        if self.error.is_none() {
            cx.emit(UserNameEvent::Confirmed {
                name,
                email,
                global: self.global,
            });
        }
        cx.notify();
    }

    fn cancel(&mut self, _: &CancelUserName, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(UserNameEvent::Cancelled);
    }

    fn next_field(&mut self, _: &NextUserNameField, window: &mut Window, cx: &mut Context<Self>) {
        let target = if self.name.focus_handle(cx).is_focused(window) {
            &self.email
        } else {
            &self.name
        };
        window.focus(&target.focus_handle(cx));
    }

    fn set(&mut self, action: &SetUserName, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(name) = &action.name {
            self.name.update(cx, |e, cx| e.set_text_selected(name, cx));
        }
        if let Some(email) = &action.email {
            self.email
                .update(cx, |e, cx| e.set_text_selected(email, cx));
        }
        if let Some(global) = action.global {
            self.global = global;
        }
        self.error = None;
        cx.notify();
    }

    pub fn state_json(&self, cx: &App) -> Json {
        json!({
            "kind": "user_name",
            "title": USER_NAME_TITLE,
            "name": self.name.read(cx).text(),
            "email": self.email.read(cx).text(),
            "global": self.global,
            "error": self.error,
            "ok": self.ok_label(),
        })
    }
}

impl Render for UserNameDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let field = |label: &'static str, editor: &Entity<Editor>, window: &Window, cx: &App| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(20.0))
                .child(div().w(px(LABEL_WIDTH)).flex_none().child(label))
                .child(editor::field::frame(editor, true, false, window, cx).flex_1())
        };
        let global = ui::form::check_row(
            "user-name-global",
            "user-name-global-check",
            self.global,
            "Set properties globally",
            cx,
        )
        .debug_selector(|| "user-name-global".into())
        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
            this.global = !this.global;
            cx.notify();
        }));
        let error = self.error.map(|e| {
            div()
                .debug_selector(|| "user-name-error".into())
                .px(px(20.0))
                .text_color(theme.color(UiRole::Danger))
                .child(e)
        });
        let buttons = BottomBar::new()
            .button(BarButton::new(
                "user-name-dialog-ok",
                self.ok_label(),
                ButtonRole::Ok,
                Box::new(ConfirmUserName),
            ))
            .button(BarButton::new(
                "user-name-dialog-cancel",
                "Cancel",
                ButtonRole::Cancel,
                Box::new(CancelUserName),
            ))
            .render(cx);
        let dialog = panel(
            "user-name-dialog",
            USER_NAME_TITLE,
            USER_NAME_WIDTH,
            false,
            cx,
        )
        .key_context(USER_NAME_KEY_CONTEXT)
        .on_action(cx.listener(Self::confirm))
        .on_action(cx.listener(Self::cancel))
        .on_action(cx.listener(Self::next_field))
        .on_action(cx.listener(Self::set))
        .child(field("Name:", &self.name, window, cx))
        .child(field("E-mail:", &self.email, window, cx))
        .child(div().px(px(20.0 + LABEL_WIDTH + 8.0)).child(global))
        .children(error)
        .child(buttons);
        frame("user-name-dialog-backdrop", dialog, window)
    }
}

#[cfg(test)]
mod focus_tests {
    use super::*;

    #[gpui::test]
    fn rollback_skips_disabled_delete_checkbox_and_button(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| ui::init(cx, themes::ThemeKind::Dark));
        let (dialog, cx) = cx.add_window_view(|_, cx| {
            RollbackDialog::new(
                Path::new("."),
                Vec::new(),
                &[],
                false,
                Rc::new(|_| IconName::FILE_TEXT),
                cx,
            )
        });
        cx.update(|window, cx| {
            window.draw(cx).clear();
            window.focus(&dialog.read(cx).focus_handle(cx));
            window.focus_next();
            window.draw(cx).clear();
        });
        let cancel = cx.update(|window, cx| window.focused(cx).unwrap());
        assert_ne!(cancel, cx.read(|cx| dialog.read(cx).focus_handle(cx)));
        // No included files: Delete Local Copies and Rollback are disabled.
        // Cancel is the only tab stop and wraps to itself both ways.
        for keys in ["tab", "shift-tab"] {
            cx.simulate_keystrokes(keys);
            assert!(cx.update(|window, _| cancel.is_focused(window)));
        }
    }
}
