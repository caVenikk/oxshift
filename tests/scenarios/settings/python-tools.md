# Python > Type Engine and Python > Tools > External Tools

## Steps
1. Open a Python project with ruff and ty installed, uv missing.
2. Open Settings > Python > Type Engine; choose Basedpyright; press Apply.
3. Open Python > Tools > External Tools.
4. Open Ruff's Lookup list and choose `Path · uvx`; turn Ruff off; browse
   for an executable of ty; press OK.
5. Click "Type Engine Settings…" in the status bar's Type Engine popup.
6. Choose the other type engine, then change Ruff's switch and Lookup on
   External Tools without applying between pages; press Apply once.
   Repeat with External Tools edited before Type Engine; reopen the project.

## Expected
- Step 2: "Type engine:" with a segmented button (ty, Basedpyright) and
  the chosen engine's description under it; Apply writes
  `[python.type-engine] name = "basedpyright"` to `.oxide/project.toml`
  and the type server restarts.
- Step 3: the description, then a framed table without a header row of
  its own (a strip with Tool, Lookup, Executable Path): Basedpyright, Ruff,
  ty, each with an ON/OFF switch, its features in green while on
  (`[Inspections, Formatting, Import optimizer]` for Ruff), the lookup
  chain with a mark per step (`Sdk ✗ · Path ✓ · uvx ✗`) and the
  executable found on the path in grey italics, or "Not found in $PATH";
  under the table the warning that uv is not installed.
- Step 4: the switch writes `enabled` (a tool turned on or off is
  explicit, ADR 0010), the lookup `lookup`; the custom executable goes to
  `.oxide/workspace.toml`; the servers are prepared again and ruff stops.
- Step 5: Settings opens on Python > Type Engine.
- Step 6: both choices survive Apply and reopening the project. Applying
  External Tools does not restore the type engine from when its page opened;
  unchanged tool fields and unknown project settings remain intact.

## Commands
`settings.open {"page": "python.type-engine"}`, `settings.edit`
(`engine`; `enabled:<tool>`, `lookup:<tool>`, `path:<tool>`, `list`),
`settings.state`, `python.tools.state`, `python.type-engine.settings`.

## Source
https://www.jetbrains.com/help/pycharm/python-type-engine.html,
https://www.jetbrains.com/help/pycharm/lsp-tools.html; IntelliJ Community
(Apache-2.0) at `pycharm/2026.2.3`: `PyTypeEngineConfigurable`,
`PyExternalToolsConfigurable`, `PyExternalToolsTable`,
`PyExternalToolsCellRenderers`, `PyExternalToolsLabels`. The page layout
is to verify against a reference screenshot.
