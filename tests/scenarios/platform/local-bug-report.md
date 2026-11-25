# User-triggered local bug report

Use a disposable project and isolated product configuration directory. Never
publish screenshots of a private project. This is a product diagnostic utility,
not an inference of the reference IDE's expected behavior.

1. Open a file and an owned Settings dialog. Invoke **Help > Create Local Bug
   Report**, or press **Cmd+Alt+Shift+B** on macOS / **Ctrl+Alt+Shift+B** on
   Windows and Linux. The action is also searchable in Find Action.
2. The IDE remains responsive. The completed report ZIP is revealed
   in the file manager and a persistent notification states the location and
   whether screenshots were saved. Nothing is uploaded.
3. Inspect `<product local data>/bug-reports/report-<timestamp>-<counter>.zip`.
   On macOS and Windows, `window-0.png` shows the requesting IDE's main window and subsequent
   images show its owned windows. Other IDE project windows and the desktop are
   absent. Existing protected-field masking remains visible as rendered.
4. Inspect `report.json`: product/version, OS/architecture, UTC time, active
   window index, main/dialog/popup roles and capture outcomes are present.
   With more than 16 owned windows, the focused window is included and the report
   records how many were omitted, with partial status. At most 20 registered command
   IDs are included; command arguments, editor text, document paths, logs and
   environment variables are absent. History belongs to this IDE process and is
   shared across its project windows, not limited to the captured window.
   Handled keyboard shortcuts are included; plain text input, query polling
   and the report commands themselves are excluded. Native menu dispatch outside
   the registry may be absent. No guessed cause, repro steps or expected result
   is generated.
5. On platforms without native capture, or when capture fails, metadata is saved
   with `status: partial`, each unavailable image has a reason, and the
   notification does not claim complete screenshot success.
6. Trigger another report. It has a different filename; the original remains
   unchanged. On Unix the report directory has mode `0700`, ZIP files `0600`.
   Temporary images and incomplete archives are removed.
7. Make the isolated report destination unwritable and trigger again. The status
   is `failed` with a useful notification; no success or upload is claimed.

8. With modal Settings open (for example from first-run), press the platform's
   report shortcut. It reaches the global report action without closing Settings
   or editing a field. The normal main-window Settings dialog is non-modal; the
   same shortcut also works there.
9. Exercise a real editor shortcut, type text, poll report state, then create a
   report. `recent_command_ids` includes only registered static command IDs of
   handled shortcuts/non-query registry invocations. It includes no typed text,
   argument values, raw keystrokes, polling or report command IDs.
10. If the file manager does not open, the full ZIP path remains in the sticky
    notification and `help.local-bug-report-state.path`. Native reveal has no
    result API, so success of opening the file manager is never claimed. A write
    failure names the intended directory in the message/state. A first-run host
    without a Workbench stack displays a native message; background automation
    uses the status query and never opens a native message.

For background automation, invoke `help.create-local-bug-report` and poll
`help.local-bug-report-state` until `saved`, `partial`, or `failed`.
Background automation does not open the file manager. The test platform returns `unavailable` without writing to the real user profile.
