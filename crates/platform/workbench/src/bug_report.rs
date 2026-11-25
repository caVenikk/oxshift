use std::fs::{self, DirBuilder};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui::{AnyWindowHandle, App, Global, Window};
use serde_json::{Value, json};

use crate::shot::CaptureTarget;

ui::ui_commands! {
    COMMANDS, scope = "Application";
    CreateLocalBugReport => "help.create-local-bug-report", "Create Local Bug Report",
        "Saves screenshots of this IDE window and its owned dialogs plus limited diagnostic metadata locally. Screenshots may contain private code. Nothing is uploaded.";
    LocalBugReportState => "help.local-bug-report-state", "Local Bug Report Status",
        "Returns the status of the latest explicitly requested local report.";
}

#[derive(Default)]
struct State {
    busy: bool,
    last: Value,
}
impl Global for State {}

pub fn init(cx: &mut App) {
    ui::command::register(cx, COMMANDS);
    ui::command::exclude_from_history(
        cx,
        &[
            "help.create-local-bug-report",
            "help.local-bug-report-state",
        ],
    );
    ui::command::observe_shortcuts(cx);
    cx.default_global::<State>();
    ui::command::register_query(
        cx,
        "help.local-bug-report-state",
        Rc::new(|_, _, cx| Ok(cx.global::<State>().last.clone())),
    );
    ui::command::register_handler(
        cx,
        "help.create-local-bug-report",
        Rc::new(|_, window, cx| {
            start(window, cx);
            Ok(cx.global::<State>().last.clone())
        }),
    );
    cx.on_action(|_: &CreateLocalBugReport, cx| {
        if let Some(active) = cx.active_window() {
            cx.defer(move |cx| {
                let _ = active.update(cx, |_, window, cx| start(window, cx));
            });
        }
    });
}

fn start(window: &mut Window, cx: &mut App) {
    if cx.global::<State>().busy {
        return;
    }
    if !crate::shot::native_windows() {
        cx.global_mut::<State>().last =
            json!({"status": "unavailable", "reason": "Test platform; no files written"});
        return;
    }
    let active = window.window_handle();
    let mut owner = active;
    while let Some(parent) =
        ui::dialog_window::parent_of(owner, cx).or_else(|| ui::popup_window::parent_of(owner, cx))
    {
        owner = parent;
    }
    // Only windows related to the requesting window. Never enumerate the desktop.
    let mut windows = vec![owner];
    let mut index = 0;
    while index < windows.len() {
        let parent = windows[index];
        let mut children = ui::dialog_window::non_modal_of(parent, cx);
        if let Some(child) = ui::dialog_window::modal_of(parent, cx) {
            children.push(child);
        }
        children.extend(ui::popup_window::windows_of(parent, cx));
        for child in children {
            if !windows.contains(&child) {
                windows.push(child);
            }
        }
        index += 1;
    }
    // Preserve the focused dialog even if a project has many open children.
    const MAX_CAPTURES: usize = 16;
    let total_windows = windows.len();
    if let Some(active_index) = windows.iter().position(|handle| *handle == active)
        && active_index >= MAX_CAPTURES
    {
        windows.swap(active_index, MAX_CAPTURES - 1);
    }
    windows.truncate(MAX_CAPTURES);
    let window_roles: Vec<_> = windows
        .iter()
        .map(|handle| {
            if let Some(parent) = ui::dialog_window::parent_of(*handle, cx) {
                if ui::dialog_window::modal_of(parent, cx) == Some(*handle) {
                    "modal_dialog"
                } else {
                    "dialog"
                }
            } else if ui::popup_window::parent_of(*handle, cx).is_some() {
                "popup"
            } else {
                "main"
            }
        })
        .collect();
    let captures = windows
        .iter()
        .map(|handle| {
            if *handle == active {
                target(window)
            } else {
                handle
                    .update(cx, |_, window, _| target(window))
                    .unwrap_or_else(|_| Err("Window closed before capture".into()))
            }
        })
        .collect::<Vec<_>>();
    let metadata = json!({
        "schema_version": 1,
        "product": app_info::full_name(),
        "version": app_info::release_version(),
        "platform": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "timestamp_utc": chrono::Utc::now().to_rfc3339(),
        "recent_command_ids": ui::command::recent_ids(cx),
        "command_history_scope": "Current IDE process; shared across its project windows, not limited to the captured window.",
        "command_history_coverage": "Non-query registry invocations and handled keyboard shortcuts. Native menu dispatch outside the registry may be absent. Diagnostics, queries, arguments and typed text are omitted.",
        "active_window": windows.iter().position(|w| *w == active),
        "window_count": total_windows,
        "window_roles": window_roles,
        "omitted_window_count": total_windows - windows.len(),
        "privacy": "User-triggered local capture. Images may contain private code. No project paths, document text, command arguments, logs or environment variables are collected as metadata. Nothing is uploaded.",
    });
    cx.global_mut::<State>().busy = true;
    cx.global_mut::<State>().last = json!({"status": "saving"});
    let root = app_info::local_data_dir().map(|p| p.join("bug-reports"));
    let destination = root
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "the unavailable local profile directory".into());
    let job = cx.background_executor().spawn(async move {
        let root = root.ok_or_else(|| "No local profile directory is available".to_string())?;
        write_report(&root, captures, metadata).map_err(|e| e.to_string())
    });
    cx.spawn(async move |cx| {
        let result = job.await;
        let _ = cx.update(|cx| {
            cx.global_mut::<State>().busy = false;
            let (kind, title, content) = match result {
                Ok((path, complete)) => {
                    cx.global_mut::<State>().last = json!({
                        "status": if complete { "saved" } else { "partial" },
                        "path": path,
                    });
                    if !crate::env::background() {
                        cx.reveal_path(&path);
                    }
                    (if complete { "information" } else { "warning" },
                     if complete { "Local bug report saved" } else { "Local bug report saved without all screenshots" },
                     format!("ZIP: {}\nIf the file manager does not open, use the path above. Screenshots may contain private code. Nothing was uploaded. See report.json inside for capture status.", path.display()))
                }
                Err(error) => {
                    cx.global_mut::<State>().last = json!({"status": "failed", "error": error, "directory": destination});
                    ("error", "Could not save local bug report", format!("Destination: {destination}\n{error}\nNo completed ZIP was created."))
                }
            };
            notify(owner, kind, title, content, cx);
        });
    }).detach();
}

fn target(window: &Window) -> Result<CaptureTarget, String> {
    if !crate::shot::native_windows() {
        return Err("Screenshots are unavailable on the test platform".into());
    }
    crate::shot::capture_target(window)
}

fn notify(owner: AnyWindowHandle, kind: &str, title: &str, content: String, cx: &mut App) {
    let _ = owner.update(cx, |_, window, cx| {
        let action = crate::notifications::Notify {
            kind: Some(kind.into()),
            title: Some(title.into()),
            content: Some(content),
            sticky: Some(true),
            actions: None,
        };
        if let Some(workbench) = window.root::<crate::Workbench>().flatten() {
            workbench.update(cx, |workbench, cx| {
                workbench.notify(action.notification(), cx);
            });
        } else if !crate::env::background() {
            // First-run/Welcome hosts may have no Workbench notification stack.
            // The native reveal API has no result; always retain the full path
            // in a visible message rather than claiming the file manager opened.
            let _ = window.prompt(
                gpui::PromptLevel::Info,
                title,
                action.content.as_deref(),
                &["OK"],
                cx,
            );
        }
    });
}

fn private_directory(path: &Path, recursive: bool) -> std::io::Result<()> {
    let mut builder = DirBuilder::new();
    builder.recursive(recursive);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

fn write_report(
    root: &Path,
    captures: Vec<Result<CaptureTarget, String>>,
    mut metadata: Value,
) -> std::io::Result<(PathBuf, bool)> {
    private_directory(root, true)?;
    if fs::symlink_metadata(root)?.file_type().is_symlink() {
        return Err(std::io::Error::other(
            "Report directory must not be a symlink",
        ));
    }
    let mut directory = tempfile::Builder::new();
    directory.prefix(".capture-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        directory.permissions(fs::Permissions::from_mode(0o700));
    }
    let temporary = directory.tempdir_in(root)?;
    let mut archive = tempfile::Builder::new()
        .prefix(".report-")
        .tempfile_in(root)?;
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o600);
    let mut zip = zip::ZipWriter::new(archive.as_file_mut());
    let mut complete =
        !captures.is_empty() && metadata["omitted_window_count"].as_u64().unwrap_or(0) == 0;
    let mut images = Vec::new();
    for (index, capture) in captures.into_iter().enumerate() {
        let name = format!("window-{index}.png");
        let image_path = temporary.path().join(&name);
        match capture.and_then(|target| target.save(&image_path)) {
            Ok((width, height)) => {
                // The PNG encoder has already compressed the frame.
                zip.start_file(
                    &name,
                    options.compression_method(zip::CompressionMethod::Stored),
                )?;
                std::io::copy(&mut fs::File::open(&image_path)?, &mut zip)?;
                images.push(
                    json!({"file": name, "status": "saved", "width": width, "height": height}),
                );
            }
            Err(error) => {
                complete = false;
                images.push(json!({"status": "unavailable", "reason": error}));
            }
        }
    }
    metadata["screenshots"] = json!(images);
    metadata["status"] = json!(if complete { "saved" } else { "partial" });
    zip.start_file("report.json", options)?;
    zip.write_all(serde_json::to_string_pretty(&metadata)?.as_bytes())?;
    zip.finish()?.sync_all()?;
    // Publish only a finished archive, exclusively. A collision never replaces
    // an earlier report. Both temporary files and images are removed on errors.
    let prefix = chrono::Utc::now().format("%Y%m%dT%H%M%S%.9fZ");
    let mut attempt = 0u32;
    loop {
        let path = root.join(format!("report-{prefix}-{attempt}.zip"));
        match archive.persist_noclobber(&path) {
            Ok(_) => return Ok((path, complete)),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                archive = error.file;
                attempt += 1;
            }
            Err(error) => return Err(error.error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_destination_does_not_claim_success_or_overwrite_a_file() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("reports");
        fs::write(&root, "keep").unwrap();
        assert!(write_report(&root, vec![], json!({})).is_err());
        assert_eq!(fs::read_to_string(&root).unwrap(), "keep");
    }

    #[test]
    fn partial_reports_are_private_and_never_overwrite_existing_data() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("reports");
        let save = || {
            write_report(
                &root,
                vec![Err("Unsupported platform".into())],
                json!({"product": "Test"}),
            )
            .unwrap()
        };
        let (first, complete) = save();
        assert!(!complete);
        let before = fs::read(&first).unwrap();
        let (second, _) = save();
        assert_ne!(first, second);
        assert_eq!(before, fs::read(&first).unwrap());
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(before)).unwrap();
        assert_eq!(zip.len(), 1);
        let report: Value = serde_json::from_reader(zip.by_name("report.json").unwrap()).unwrap();
        assert_eq!(report["status"], "partial");
        assert_eq!(report["screenshots"][0]["status"], "unavailable");
        assert!(zip.by_name("window-0.png").is_err());
        assert_eq!(
            fs::read_dir(&root).unwrap().count(),
            2,
            "only final archives remain"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(first).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}
