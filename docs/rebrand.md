# Rebranding

Oxshift is the shared platform. Oxshift Python is its first IDE. Its display
names and compatibility identifiers are defined in the `brand` crate
(`crates/python/brand/src/lib.rs`, product layer). See the
[brand guide](../replica/brand.md) for the approved icons and palettes.

- Product crates in `crates/python/` use the `brand` constants and
  `brand::env_name!("X")` (compile time, for `concat!` and attributes).
- Platform crates and modules know no name (ADR 0005). They read the
  `app_info::AppInfo` the product installs at startup (`brand::install()` in
  `main`, `Product::app_info` in `workbench::run`); until then it is a neutral
  one (`App`, slug `app`, prefix `APP`, folder `.app`), which is what the tests
  of platform crates run with.
- `python3 scripts/check-brand.py` (part of `scripts/check.sh` and CI) fails on
  either the current or previous product name in the code of `crates/` outside `brand`, apart from comments, test
  code, `Cargo.toml` and a short allow-list in the script, each entry with its
  reason.

## What `brand` holds

| Constant | Today | Where it shows up |
|---|---|---|
| `PRODUCT_NAME` | `Oxshift Python` | window title, menus, messages and `TERMINAL_EMULATOR` |
| `IDENTITY_NAME` | `Oxshift` | KeePass group |
| `PRODUCT_FULL_NAME` | `Oxshift Python IDE` | LSP and DAP client name, Oracle session module, `--help` |
| `SLUG` | `oxshift` | Linux window app ID, database `application_name`, thread names, temporary files, base theme ids (`oxshift-dark`, `oxshift-light`), save-file marker |
| `ENV_PREFIX` | `OXSHIFT` | environment variables, including `OXSHIFT_CONFIG_DIR` and `OXSHIFT_DB_PASSWORD`, and the `OXSHIFT ` measurement-line prefix |
| `PROFILE_DIR` | `oxshift-python` | profile in the OS configuration and cache locations |
| `PROJECT_DIR` | `.oxshift` | shared and personal project settings, run configurations and project data sources |
| `BUNDLE_ID` | `dev.oxshift.python` | application bundle identifier |
| `CLI_NAME`, `DB_CLI_NAME`, `IDE_NAME` | `oxshift`, `oxshift-db`, `oxshift-python` | commands and help text. The credential service is `oxshift-db`. |

## Project storage and PyCharm import

The standalone desktop uses the `oxshift-platform` profile, the
`dev.oxshift.platform` bundle ID and the Oxshift star icon. The Python desktop
uses the Oxshift Python snake icon. The two applications can run side by side.

The Python IDE and both command-line tools use `oxshift-python/` in the OS
configuration and cache locations. Profiles and caches from the previous
product identity are not selected or imported automatically. Explicit configuration and cache path
overrides select the requested location instead of the default.

Project settings always use `.oxshift/`, which holds `project.toml`,
`workspace.toml`, shared run configurations and project data sources together.
This directory is hidden from the project tree and index. There is no fallback
to the former project directory and no automatic migration of it. Supported
PyCharm project settings are imported from `.idea/` and shared `*.run.xml`
files by their respective importers. The first-run import of personal PyCharm
settings keeps a snapshot of supported configuration data; credentials,
password stores and tokens are excluded. Explicit settings and cache paths
still override the default. CLI database profiles and the secret-store
configuration retain their own overrides, `OXSHIFT_DB_PROFILES` and
`OXSHIFT_SECRETS_CONFIG`.

The base themes are `oxshift-dark` (Oxshift Python Dark) and
`oxshift-light` (Oxshift Python Light). Credential services are `oxshift-db` (OS keychain) and
`oxshift-keepass`, with KeePass group `Oxshift`. Signing uses Oxshift identifiers
and the local certificate `Oxshift Local Dev` (see [Code signing](code-signing.md)).
Credentials from the previous product identity are not moved automatically.
A user can enter a saved password again in Oxshift; the previous credential
remains safely available in its original store.

## Rollout

1. **Names on screen and in public copy.** Use Oxshift for the platform and
   Oxshift Python for the IDE. The application menu, welcome screen, CLI help,
   README, and feature progress page take the new name. Examples use the current package names, repository URL, and environment variables.
2. **Commands and packages.** The primary packages are `oxshift-python`
   (IDE), `oxshift-cli` (the `oxshift` executable), and `oxshift-db`. Use these
   package names for builds, runs and tests. Commands and environment variables
   use only the Oxshift names: `oxshift-python`, `oxshift`, `oxshift-db`, and
   `OXSHIFT_*`. There are no old command wrappers or environment aliases.
   CLI flags take precedence over environment defaults. The applications use
   the Oxshift settings locations; credential stores use the identifiers above.
3. **Settings, projects, and credentials.** Use the product storage paths
   and import supported PyCharm settings as described above.
   Distribution signing and packaged Keychain acceptance are release checks.
4. **Repository and documentation.** The canonical repository is
   [caVenikk/oxshift](https://github.com/caVenikk/oxshift). Existing clones can
   update their SSH remote with `git remote set-url origin git@github.com:caVenikk/oxshift.git`.
   The [feature report](https://cavenikk.github.io/oxshift/) and its
   [badge data](https://cavenikk.github.io/oxshift/parity.json) use the new Pages path.
5. **Distributions.** Apply the Python icon from `assets/brand/` and the product
   name to the macOS bundle, Windows executable and installer, and Linux desktop
   entry. Align installed binary names, bundle identifiers, and CI artifact
   names with the migration decisions. Check install and upgrade on each OS.
