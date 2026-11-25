# One-time pick-up of the reference IDE's settings

## Steps
1. Start Oxshift Python with an empty configuration directory
   (`OXSHIFT_CONFIG_DIR=<empty dir>`) on a machine where the reference
   IDE's configuration has a non-default theme and scheme (a theme
   plugin), editor font size 14 with ligatures on, soft wraps off, a
   custom keymap based on macOS and two interpreters (one of them deleted
   from disk).
2. Choose the newest configuration ("Import Settings", the folder's button
   names its version).
3. Look at the theme, an editor, soft wraps, a few shortcuts of the
   custom keymap and the interpreter list.
4. Look at `settings.toml` and `imported/` in the configuration directory.
5. Restart Oxshift Python.
6. Start once more with an empty configuration and choose "Skip Import".

## Expected
- Step 1: the first-run window comes before the main window, modal, on
  "Import Settings": the newest configuration found, "Skip Import", and
  "Other Options" with the older ones and "Config or Installation
  Directory" (each folder in two lines with an icon: the name over when
  it was used; the list at least as wide as the button), over a teal and
  olive glow. Without a configuration to offer no window comes (the
  Welcome tab offers the theme and the keymap, see welcome.md).
- Step 2: "Importing settings…", then the window closes and the main
  window opens with a "Settings imported" balloon naming what came in and
  what could not; its Welcome tab shows the imported theme and keymap.
- Step 3: theme, scheme, font, ligatures, soft-wrap setting and keymap
  match the reference; only the interpreter that still exists is listed
  (the configured interpreter list, #75, takes it from the snapshot's
  `options/jdk.table.xml` with the importer `python-interpreters`, #243).
- Step 4: `[import]` has `source`, `version`, `time` and
  `snapshot = "imported/<source>-<version>"`; `[import.applied]` lists the
  importers that ran. The snapshot holds the data files (`options/*.xml`,
  `keymaps/`, `colors/`, `codestyles/`, `templates/`), never
  `security.xml`, `c.kdbx`, `c.pwd`, licences or tokens, and no plugins.
- Step 5: no second import; an area added in a newer version takes its
  part from the snapshot once and is added to `[import.applied]`.
- Step 6: defaults (Islands theme, the platform's keymap); `[import]`
  has `source = "skipped"` and no snapshot is kept.

## Commands
`first-run.state`, `first-run.import`, `first-run.skip`,
`first-run.other-options`, `settings.import-reference`, `settings.get`.

## Source
https://www.jetbrains.com/help/pycharm/sharing-your-ide-settings.html;
the reference's data files `laf.xml`, `colors.scheme.xml`, `keymap.xml`,
`editor-font.xml`, `editor.xml`, `ui.lnf.xml`, `ide.general.xml`,
`filetypes.xml`, `jdk.table.xml` and theme plugin descriptors (formats
read in IntelliJ Community at `pycharm/2026.2.3`); the first-run flow of
`intellij.ide.startup.importSettings` at the same tag.


## Source identity and selection (#501)

1. Supply recent `PyCharm2026.2` and `PyCharmCE2025.3` configurations and an
   older `PyCharm2025.1` configuration. Include unrelated `GoLand2026.2`,
   `DataGrip2026.2` and `IntelliJIdea2026.2` folders alongside them.
2. On first launch, open the PyCharm source selector; use Up/Down and Enter to
   choose a version. Repeat with Other Options for the older configuration.
3. Check `first-run.state`, the progress source and the imported notification.
4. Reopen via `settings.import-reference`; choose a manual folder called
   `my-settings` (with an `options/` subfolder), then a `PyCharmBackup` folder.
5. Repeat with light and dark themes and a theme that changes icon colours.
6. Restart with the saved profile; compare the snapshot and imported values.

Expected: supported sources show `PyCharm 2026.2`,
`PyCharm Community Edition 2025.3` and `PyCharm 2025.1` consistently, including
command state and keyboard-selectable menu text. Unrelated products are absent.
Manual unknown folders use `Settings folder: <name>` and a neutral icon, never
a guessed PyCharm identity. The official icon is unmodified, at least 24 px,
with padding, and accompanied by attribution and a JetBrains link; theme changes
do not recolour it. Import, skip, cancellation, snapshot privacy and one-time
pick-up retain the behavior above. The application title remains Oxshift Python.

The Python product explicitly enables the PyCharm source in its init hook. A
neutral workbench host with the same fixture folders finds no automatic import
source and registers no PyCharm icon; manually requested imports are refused.
