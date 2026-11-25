# Oxshift

Oxshift is a native desktop IDE platform written in Rust. It provides the shared
editor, workbench, project model, command registry, settings, version control,
terminal and database services used by language-specific products. The UI uses
GPU rendering through GPUI, with no JVM or web view.

## Desktop products

**Oxshift** is also available as a standalone desktop app. It includes the editor,
project tree, Git, terminal, shell run configurations and Python and SQL syntax
highlighting. Run it alongside the Python IDE with its own preferences. Both
products share the 2026.1 release line.

[**Oxshift Python IDE**](docs/oxshift-python.md) is the first product built on
Oxshift. It combines Python tools, debugging, test execution and database
workflows. It is **pre-alpha**: check the [feature matrix](replica/features/) and
[roadmap](docs/ROADMAP.md) for the current implementation and planned work.
Other language IDEs are not currently available.

### Install

Download the package for your product and operating system from
[GitHub Releases](https://github.com/caVenikk/oxshift/releases).
On macOS, open the DMG and drag the application into Applications. On Windows
x64, use the installer or extract the ZIP and run the product executable.
Linux packages are available for x86_64 and arm64. See the
[release guide](docs/release.md) for package formats, checksums and signing.

To run either product from source:

```bash
git clone https://github.com/caVenikk/oxshift && cd oxshift
cargo run --release -p oxshift-platform -- path/to/project
cargo run --release -p oxshift-python -- path/to/project
```

The GUI binaries are `oxshift-platform` and `oxshift-python`. The additional programs are `oxshift` (Python IDE CLI)
and `oxshift-db` (database CLI). See the [Python IDE guide](docs/oxshift-python.md)
for build instructions and usage, and the [database CLI guide](docs/oxshift-db.md)
for database commands.

## Local bug reports

In either desktop app, choose **Help → Create Local Bug Report**. The shortcut is
**⌘⌥⇧B** on macOS and **Ctrl+Alt+Shift+B** on Windows. The app reveals a ZIP in
the product’s local data folder. On Windows this is
`%LOCALAPPDATA%/<profile>/bug-reports` and on macOS it is
`~/Library/Application Support/<profile>/bug-reports`. The profile is
`oxshift-platform` or `oxshift-python`. The ZIP contains diagnostic metadata
and, when available, images of the requesting window and its open dialogs or
popups. The notification indicates if a capture failed. Images can show code
and other visible information. You choose whether to share the archive.

## Platform and products

- `crates/platform/`: language-independent engine services and desktop UI.
- `crates/python/`: Python-specific tools, product branding and entry points.
- `crates/languages/`: shared syntax support.
- `crates/desktop/`: the standalone desktop product.
- `crates/database/`: the database module shared through platform interfaces.

Platform crates do not depend on product crates. Products compose the shared
services and provide their language tools and identity. See
[Architecture](docs/architecture.md) and [ADR 0005](docs/adr/0005-platform-and-language-products.md).
Performance targets and measurement rules are in [Performance](docs/performance.md).

## Progress and documentation

[![Feature parity](https://img.shields.io/endpoint?url=https%3A%2F%2Fcavenikk.github.io%2Foxshift%2Fparity.json)](https://cavenikk.github.io/oxshift/)

The [Python IDE feature report](https://cavenikk.github.io/oxshift/) is generated
from [`replica/features/`](replica/features/) after changes on `main`.
`python3 scripts/progress.py` prints it locally.

- [Oxshift Python IDE](docs/oxshift-python.md) · [Roadmap](docs/ROADMAP.md)
- [Database CLI](docs/oxshift-db.md) · [Database testing](docs/testing-databases.md)
- [Architecture](docs/architecture.md) · [Decisions](docs/adr/) · [Brand and storage](docs/rebrand.md)
- [Contributing](CONTRIBUTING.md) · [Agent workflow](docs/agent-workflow.md) · [AGENTS.md](AGENTS.md)
- [Clean-room rules](docs/clean-room.md) · [Theme formats](docs/theme-formats.md)

## Contributing

Follow [CONTRIBUTING.md](CONTRIBUTING.md), [AGENTS.md](AGENTS.md) and the
[clean-room rules](docs/clean-room.md). Development uses public documentation,
open specifications, compatible open-source code and observed behaviour.
The development skill pack lives in [`skills/`](skills/OXSHIFT.md).
Every user-visible action is a registered command shared by the UI, CLI and
agent control channel.

## License

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at the
licensee's option. Contributions submitted for inclusion use the same terms.
Third-party notices are in [NOTICE](NOTICE).

## Trademarks

Oxshift is an independent project, not affiliated with, sponsored by or endorsed
by JetBrains s.r.o., the Python Software Foundation or Oracle Corporation.
PyCharm, IntelliJ, JetBrains, Python and Oracle are trademarks of their respective
owners and are used only to describe compatibility.
