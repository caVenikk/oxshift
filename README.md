# Oxshift Python

Oxshift Python is a native Python IDE built on the Oxshift platform in Rust.

Editor, debugger, test runner, version control and a built-in **database client**
in one lightweight app, with no JVM and no web view. Offline, open source, and
scriptable: every action is a command, so the UI, the CLI and AI agents drive
the same engine.

> **Pre-alpha.** Run the IDE with `cargo run --release -p oxshift-python -- <folder>`.
> The commands are `oxshift-python` (IDE), `oxshift` (CLI), and `oxshift-db`
> (database client). See the [feature matrix](replica/features/) for implemented features
> and the [roadmap](docs/ROADMAP.md) for upcoming work.

## Goals

- **Speed first.** Native Rust, GPU rendering, everything off the UI thread.
  Performance budgets are tracked in CI ([docs/performance.md](docs/performance.md)).
- **Python-focused.** The Python product provides language tools and debugging.
  The Oxshift platform provides the shared editor and workbench.
- **Standing on good tools.** Python intelligence comes from existing Rust/LSP
  tooling (ruff, a type checker such as ty, basedpyright or pyrefly) and
  debugging from debugpy over DAP, instead of a hand-written analyzer.
- **Windows is a first-class platform**, together with macOS and Linux.
- **Familiar to PyCharm users**: import of keymaps, code styles, live
  templates, editor colour schemes (`.icls`) and UI themes (`.theme.json`).
- **Clean-room.** Built from public documentation, open specifications,
  permissively licensed code and observed behaviour only. See
  [docs/clean-room.md](docs/clean-room.md).

## Progress

[![Feature parity](https://img.shields.io/endpoint?url=https%3A%2F%2Fcavenikk.github.io%2Foxshift%2Fparity.json)](https://cavenikk.github.io/oxshift/)

Feature parity with PyCharm, overall, per area and per feature, is published at
[cavenikk.github.io/oxshift](https://cavenikk.github.io/oxshift/) after
every change on `main`. It is generated from the feature matrix in
[`replica/features/`](replica/features/), one file per feature;
`python3 scripts/progress.py` prints the same report locally.

## Workspace

Crates live in `crates/platform/` (language-agnostic) and `crates/python/`
(the Python product); see ADR 0005.

| Crate | Purpose |
|---|---|
| `commands` | Command registry shared by the UI, the CLI and the agent control channel |
| `db-core` | Backend-agnostic traits, typed values, cursors, cancel/timeouts, schema introspection |
| `db-oracle-thin` | Oracle over pure Rust (`oracle-rs`), no Oracle Client needed |
| `db-oracle-odpi` | Oracle through an installed Oracle Client (ODPI-C), fallback |
| `db-postgres` | PostgreSQL (`tokio-postgres`, rustls) |
| `db-sqlite` | SQLite (`rusqlite`, bundled) |
| `db-service` | Connection profiles, diagnostics, PyCharm import, `db.*` commands |
| `db-testkit` | Conformance suite every database driver runs |
| `secrets` | Secret stores: OS credential store, KeePass, memory |
| `settings-import` | Import of PyCharm project settings (data sources) |
| `themes` | Theme model, default themes, import of PyCharm colour schemes and UI themes |
| `text` | Rope text buffer, selections, undo/redo, line endings and encodings |
| `syntax` | tree-sitter parsing and highlighting off the UI thread |
| `project` | Project folder, lazy file tree, reading and saving documents |
| `ui` | Component layer on GPUI: theme, fonts, icons, UI commands |
| `editor` | The code editor view: layout, carets, selections, input, editing commands |
| `workbench` | Main window: toolbar, tool windows, tabs, status bar, keymaps |
| `py-syntax` (python) | Python grammar, highlight query and editing rules |
| `oxshift-python` (python) | `oxshift-python`, the Python product's application |
| `py-env` (python) | Interpreters, virtualenvs, uv and pip integration, `env.*` commands |
| `oxshift-cli` (python) | `oxshift`, the Python product's command line (`env`, plus all `db.*` commands) |
| `oxshift-db` | `oxshift-db`, a command-line database client |

## Getting started

The IDE window (the default keymaps and what works so far:
[tests/scenarios/editor/keymap-and-editing-basics.md](tests/scenarios/editor/keymap-and-editing-basics.md)):

```bash
git clone https://github.com/caVenikk/oxshift && cd oxshift
cargo run --release -p oxshift-python -- path/to/project
```

Run `cargo build --release -p oxshift-python -p oxshift-cli -p oxshift-db` to
build all three current programs.

Project settings are stored in `.oxshift/`. Supported PyCharm project settings
are imported from `.idea/`; personal PyCharm settings can be imported on first run.
User settings and caches use the OS `oxshift-python/` locations. Existing Pyoxide
profiles and project settings are not imported automatically.
See [project storage and PyCharm import](docs/rebrand.md#project-storage-and-pycharm-import).

The database client:

```bash
docker compose up -d            # PostgreSQL 18 and Oracle Free 23 for development
export OXSHIFT_DB_PASSWORD=pyoxide
cargo run -p oxshift-db -- --service FREEPDB1 --user pyoxide test
cargo run -p oxshift-db -- --backend postgres --port 55432 --database pyoxide --user pyoxide query "select now()"
cargo run -p oxshift-db -- --path :memory: query "select sqlite_version()"
cargo run -p oxshift-db -- --service FREEPDB1 --user pyoxide diagnose
```

Passwords come from the environment, the OS credential store or a prompt,
never from arguments. Profiles, secrets and diagnostics:
[docs/oxshift-db.md](docs/oxshift-db.md).

Python environments (needs [uv](https://docs.astral.sh/uv/); details in
[docs/python-environments.md](docs/python-environments.md)):

```bash
cargo run -p oxshift-cli -- env interpreters
cargo run -p oxshift-cli -- env info .
cargo run -p oxshift-cli -- commands
```

Import report for a PyCharm colour scheme, UI theme or theme plugin:

```bash
cargo run -p themes --example import -- path/to/theme.jar
```

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace     # cargo install cargo-nextest --locked
cargo test --doc --workspace      # nextest does not run doctests
cargo test -p db-postgres -p db-oracle-thin -- --ignored   # needs docker compose up
```

## Documentation

- [Roadmap](docs/ROADMAP.md) · [Architecture](docs/architecture.md) · [Performance](docs/performance.md)
- [oxshift-db](docs/oxshift-db.md) · [Testing against databases](docs/testing-databases.md) · [Decisions (ADR)](docs/adr/)
- [Clean-room rules](docs/clean-room.md) · [Theme formats](docs/theme-formats.md)
- [Contributing](CONTRIBUTING.md) · [Agent workflow](docs/agent-workflow.md) · [AGENTS.md](AGENTS.md)

## Built for agents

The repository is set up for AI-assisted development: [AGENTS.md](AGENTS.md)
holds the rules, `skills/` holds the development skill pack (see
[docs/agent-workflow.md](docs/agent-workflow.md)), and every user-visible action
is a registered command that agents can call.

## Contributing

Issues and pull requests follow [CONTRIBUTING.md](CONTRIBUTING.md) and the
[clean-room rules](docs/clean-room.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at the licensee's option. This is the common licensing of the Rust ecosystem:
Apache-2.0 carries an explicit patent grant, MIT keeps compatibility with
GPL-2.0 projects. Contributions submitted for inclusion are licensed under the
same terms, without additional conditions. Third-party notices are in
[NOTICE](NOTICE).

## Trademarks

Oxshift Python is an independent project, not affiliated with, sponsored by or
endorsed by JetBrains s.r.o., the Python Software Foundation or Oracle
Corporation. PyCharm, IntelliJ, JetBrains, Python and Oracle are trademarks of
their respective owners and are used only to describe compatibility.
