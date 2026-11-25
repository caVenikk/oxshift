# Release packaging

The release line is **2026.1** (`2026.1.0` in Cargo and bundle build metadata,
`v2026.1` as the release tag). `scripts/package.py` reads the current product
identities and workspace version. It refuses a version that disagrees with the
workspace. It packages existing binaries; it never builds, tags, uploads or
publishes by itself.

## Products and artifacts

Every target builds both products once:

| Product | Artifact prefix | Programs | Application bundle |
|---|---|---|---|
| Oxshift | `oxshift-platform` | `oxshift-platform` | `Oxshift.app` |
| Oxshift Python | `oxshift-python` | `oxshift-python`, `oxshift-db`, `oxshift` | `Oxshift Python.app` |

Each uses its own canonical artwork from `assets/brand/`. Both macOS packagers
call the same `macos_icons.py` Icon Composer function: `.icon` input, `Assets.car`
and `.icns` output, and `CFBundleIconName` metadata. No static-icon fallback is
allowed. The two macOS 15 runner images explicitly select Xcode 26.3 with Icon
Composer support; a missing toolchain fails the job. Availability is recorded in
the official [Intel](https://github.com/actions/runner-images/blob/main/images/macos/macos-15-Readme.md)
and [Apple silicon](https://github.com/actions/runner-images/blob/main/images/macos/macos-15-arm64-Readme.md)
runner image inventories. Windows/Linux use the
canonical `.ico`/`.png` artwork.
The macOS bundle identifiers are `dev.oxshift.platform` and `dev.oxshift.python`.
Only the Python product includes the pinned, unmodified debugpy helper and its
third-party notices. Both Windows products include the pinned ConPTY runtime
beside the executable. All packages include project and font license material.

| Native runner target | Formats | Boundary |
|---|---|---|
| macOS Apple silicon (`aarch64`) | `.dmg`, `.tar.gz` | Ad hoc signed; not notarized |
| macOS Intel (`x86_64`) | `.dmg`, `.tar.gz` | Separate native build; no universal binary |
| Linux `x86_64`, `aarch64` | `.tar.gz`, `.deb`, `.AppImage` | Built on Ubuntu 24.04; glibc 2.39 or newer |
| Windows `x86_64` | `.setup.exe`, `.zip` | Unsigned Inno Setup installer; archive has portable marker |

Names have the form `oxshift-python-2026.1-macos-aarch64.dmg`. Windows ARM64
is not produced: the pinned ConPTY payload is x64. Installer compatibility
with x64 emulation is not a native ARM64 claim. macOS bundles declare macOS 13
as their floor; execution on older OS versions than the build runners remains
a separate acceptance check.

The AppImage contains the application payload and a pinned type-2 runtime. It
still relies on the host's graphics libraries: fontconfig, FreeType, X11/XCB,
xkbcommon, Wayland and Vulkan. This is not a claim of universal Linux support.
Make an AppImage executable with `chmod +x file.AppImage` before opening it.
AppImage tool 1.9.1 and runtime 20251108 are checked by SHA-256 before use; the
explicit `--runtime-file` prevents an unpinned runtime download. The Debian
package declares these native dependencies. Native runner package creation,
container inspection and user installation acceptance must all pass before a
format is described as tested.

## Preview first, publish separately

`.github/workflows/desktop-packages.yml` is a reusable `workflow_call` workflow
and can also be dispatched manually. Its jobs have `contents: read`, build the
five targets, check packaging fixtures, verify archive contents/checksums and
inspect DMG, Debian and AppImage containers. It uploads **Actions artifacts**,
not release assets. Windows archive smoke reports are separate sanitized JSON
in `evidence-<prefix>-windows`; absence of a desktop/GPU is reported as unavailable,
not as a successful GUI test. It never creates a tag, GitHub Release, tap commit or public
package registry entry.

Inputs:

- `release_version`: empty uses the workspace calendar version.
- `artifact_prefix`: defaults to `desktop-packages`.
- `retention_days`: defaults to 14.

Outputs are artifacts named `<prefix>-macos-aarch64`, `-macos-x86_64`,
`-linux-aarch64`, `-linux-x86_64`, `-windows-x86_64`, and `-manifest`.
The manifest contains `SHA256SUMS`, per-product notes, Homebrew formulas and
inspectable user-install scripts. Each native artifact also has a checksum
sidecar. Checksums detect corruption; an unsigned checksum file is not a
publisher signature.

After full CI and maintainer build acceptance, a maintainer may create and push
`v2026.1`. The separate `release.yml` checks that the tag matches the version,
the exact SHA belongs to actual `main`, its latest exact `ci.yml` main push
succeeded, and repository rules hold. The gate accepts a full three-OS/database
run or the existing verified-main reuse branch: for reuse it revalidates the
immutable full PR receipt with `ci-control.py`, binding exact tree, base/head,
configuration, toolchain, workflow and three-OS/database results. Arbitrary
named checks, draft or skipped runs and unrelated dispatches are insufficient. It calls the same artifact workflow and creates a **draft** release
with `--verify-tag`. It never publishes that draft or replaces an existing
release. The maintainer reviews it and publishes separately. No paid signing
credentials or token for another repository are assumed.

Before tagging, review the MVP feature matrix, all three OS test results and
actual installation checks. Packaging fixture success alone is not runtime or
installer acceptance. The exact release commit must include the Windows
portable runtime support described below; adding a marker alone cannot route
application state.

## Local commands

Build using the existing native prerequisites in CI, then package on that OS:

```sh
cargo build --release --locked -p oxshift-platform -p oxshift-python -p oxshift-cli -p oxshift-db
python3 scripts/package.py build --product all --out dist --formats tar.gz,dmg
```

On Linux:

```sh
python3 scripts/fetch-appimage-tools.py --dest build-tools
python3 scripts/package.py build --out dist --formats tar.gz,deb,AppImage \
  --appimagetool "$PWD/build-tools/appimagetool" --appimage-runtime "$PWD/build-tools/runtime"
```

On Windows (PowerShell, Inno Setup 6 installed):

```powershell
python scripts/package.py build --out dist --formats zip,installer --iscc "C:/Program Files (x86)/Inno Setup 6/ISCC.exe"
```

`--debugpy DIR` and `--conpty DIR` use already prepared helpers, otherwise the
existing pinned fetchers download and verify them. `--sign-identity` and
`--sign-keychain` select a local macOS identity; the default is ad hoc signing.
The output directory must not contain an artifact with the same name.

```sh
python3 scripts/test-package.py
python3 scripts/package.py check
python3 scripts/package.py verify dist/oxshift-python-2026.1-linux-x86_64.tar.gz --product python
python3 scripts/package.py sums dist
```

`verify` checks payload and checksum without opening a GUI. `--smoke-cli` also
runs the Python product's native CLI `--version`; it is not a desktop smoke test.
Fixture tests use fake executables and mocked native tools. They cover product
separation, version mismatch, archives, helper notices, Python bytecode exclusion,
Windows marker/secret seed/installer exclusion, bundle identities/icons, native
layout commands, checksums and rendered metadata. They do not certify native
signatures, installers, OS integration or startup.

## Installing and operating-system prompts

For macOS, drag the app from the DMG to Applications. These builds are not
notarized: macOS can block a quarantined download. Use the operating system's
Privacy & Security **Open Anyway** flow after checking the source and checksum.
The alternative per-product install script checks the archive checksum and
installs into `~/Applications` (Linux: `~/.local/share`). It preserves macOS quarantine. The installer records ownership so that a later
update or uninstall can identify its application and launchers.
Homebrew formulas are rendered for each product; no tap is created or updated
automatically.

Windows installers offer current-user installation without administrator rights
and all-users installation with elevation, separate application locations and
an uninstaller. File associations are not installed. SmartScreen may block
unsigned applications. Enterprise AppLocker/WDAC policy may forbid programs in
user folders; an installer cannot bypass that policy.

The Windows ZIP contains `data/.oxshift-portable` and seeds
`data/config/secrets.toml` with `store = "memory"`. With portable runtime support,
application profiles use `data/config`, caches `data/cache`, local bug reports
`data/bug-reports`, and Git's transient message/pathspec files `data/tmp`.
Memory-only secrets disappear when the application exits. Persistent secrets
require deliberate configuration, for example a KeePass file and its master
password source. The normal installer contains no `data` subtree and keeps the
normal OS profile/credential behavior.

Portable mode is not a sandbox. Saving project files, `.oxshift` project data,
Git operations and selected exports write to the chosen project/destination.
Shells, Python, language servers, debug adapters, Git hooks, database libraries,
Windows crash reporting and other OS services have their own storage behavior.
ConPTY is loaded by `alacritty_terminal` using the Windows loader, with system
fallback; its presence beside the binary does not constrain its internal writes.
No claim is made that all activity stays inside `data`.
