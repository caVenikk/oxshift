# Open the terminal with the project's virtual environment

## Steps
1. Open a uv project that has a `.venv`; press Option+F12 (macOS) or
   Alt+F12 (Windows, Linux).
2. Run `which python` (`where python` on Windows), `echo $VIRTUAL_ENV`
   and `pwd`.
3. Press Option+F12 / Alt+F12 twice.
4. Turn "Activate virtualenv" off in Settings > Tools > Terminal and open
   a new tab.
5. Open a project whose interpreter is a system Python and open the
   terminal.
6. In a project A with its own `.venv`, select a registered virtual
   environment from project B as A's interpreter and open a new tab.
   Repeat with B first registered through its `bin/python3` executable.
7. Select A's own interpreter and open another tab, then open a terminal
   in B. Turn off "Activate virtualenv" in A and open another tab there.

## Expected
- Step 1: the Terminal tool window opens at the bottom with one tab
  "Local" and the focus in it; the shell is the user's login shell
  (interactive, and a login shell on macOS); the prompt shows the
  environment's prefix, for example `(.venv)` or the uv project name.
- Step 2: python is the environment's interpreter inside `.venv`,
  `VIRTUAL_ENV` points to `.venv`, the working directory is the project
  root.
- Step 3: the first press hides the tool window, the second shows the
  same tab with its output.
- Step 4: the new tab has no environment prefix and no `VIRTUAL_ENV`.
- Step 5: no environment is activated.
- Step 6: `VIRTUAL_ENV`, `which python` and Python's `sys.executable` point
  into B's environment; `pwd` remains A. This also applies to shells
  without integration scripts, which receive activation variables.
- Step 7: A's new tab uses A's environment; B still uses B's. Existing
  tabs keep their running shells. With activation off, a new tab does
  not activate the selected environment.

The ignored `terminal_venv` integration tests use real uv environments,
zsh and tcsh through the product's SDK-selection and terminal commands.

## Commands
`workbench.toggle-terminal`, `terminal.new-tab`,
`terminal.type {"text": ...}`, `terminal.state` (tabs, titles, working
directory, visible lines, cursor).

## Source
https://www.jetbrains.com/help/pycharm/terminal-emulator.html,
https://www.jetbrains.com/help/pycharm/settings-tools-terminal.html;
IntelliJ Community (Apache-2.0) at `pycharm/2026.2.3`:
`LocalTerminalStartCommandBuilder`, `LocalOptionsConfigurer`,
`PyVirtualEnvTerminalCustomizer`. The prompt and header look are to verify
against a reference screenshot.
