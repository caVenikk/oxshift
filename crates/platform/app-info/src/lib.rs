//! The identity of the running product, as the platform sees it (ADR 0005).
//!
//! Platform crates know no product name. The product installs an [`AppInfo`]
//! once at startup ([`install`]); until then (in the tests of a platform
//! crate, for example) [`get`] returns a neutral one. Names that users see
//! are formatted from [`name`], technical names (configuration and cache
//! folders, keychain services, environment variables, the project folder)
//! from [`AppInfo::profile_dir`], [`slug`], [`env_prefix`] and [`project_dir`].

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The names of a product. Every field is a plain value the product owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppInfo {
    /// Short name in sentences and menus.
    pub name: &'static str,
    /// Full name where the short one is too vague (window titles, user agents).
    pub full_name: &'static str,
    /// Stable, case-sensitive name for existing credential groups and shell identity.
    /// Independent of the display name; changing it requires a migration.
    pub identity_name: &'static str,
    /// Stable technical name for database sessions and thread names.
    pub slug: &'static str,
    /// Directory name under the OS configuration and cache locations.
    pub profile_dir: &'static str,
    /// Prefix of the environment variables, without the trailing `_`.
    pub env_prefix: &'static str,
    /// Previous environment prefix, accepted only when the canonical variable is absent.
    pub legacy_env_prefix: Option<&'static str>,
    /// Stable prefix of machine-readable measurement output.
    pub measurement_prefix: &'static str,
    /// Name of the product's folder in a project root, with the leading dot.
    pub project_dir: &'static str,
    /// Reverse-DNS identifier of the application bundle.
    pub bundle_id: &'static str,
    /// Name of the command-line program.
    pub cli_name: &'static str,
    /// Name of the database command-line program.
    pub db_cli_name: &'static str,
    /// Names of each program sharing credentials, preferred name first, then aliases.
    pub binaries: &'static [&'static [&'static str]],
}

impl AppInfo {
    /// The names used when the product installed none.
    pub const NEUTRAL: AppInfo = AppInfo {
        name: "App",
        full_name: "App",
        identity_name: "App",
        slug: "app",
        profile_dir: "app",
        env_prefix: "APP",
        legacy_env_prefix: None,
        measurement_prefix: "APP",
        project_dir: ".app",
        bundle_id: "app.example.app",
        cli_name: "app",
        db_cli_name: "app-db",
        binaries: &[&["app"], &["app-db"]],
    };
}

impl AppInfo {
    /// The project store path, without probing or creating directories.
    pub fn project_storage_dir(&self, root: &std::path::Path) -> PathBuf {
        root.join(self.project_dir)
    }

    /// Resolve a full variable name against an injected environment. Canonical
    /// presence wins even for an empty value. Unrelated names are read verbatim.
    /// Return the actual name for diagnostics; callers must never print values.
    pub fn lookup_env<T>(
        &self,
        name: &str,
        lookup: impl Fn(&str) -> Option<T>,
    ) -> Option<(String, T)> {
        let canonical_prefix = format!("{}_", self.env_prefix);
        let legacy_prefix = self.legacy_env_prefix.map(|p| format!("{p}_"));
        let suffix = name
            .strip_prefix(&canonical_prefix)
            .or_else(|| legacy_prefix.as_deref().and_then(|p| name.strip_prefix(p)));
        let Some(suffix) = suffix else {
            return lookup(name).map(|value| (name.to_owned(), value));
        };
        let canonical = format!("{canonical_prefix}{suffix}");
        if let Some(value) = lookup(&canonical) {
            return Some((canonical, value));
        }
        let legacy = format!("{}{suffix}", legacy_prefix?);
        lookup(&legacy).map(|value| (legacy, value))
    }
}

impl Default for AppInfo {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

static INFO: OnceLock<AppInfo> = OnceLock::new();

/// Installs the product's names. The first call wins; returns whether this
/// call installed them (a second call with other names changes nothing).
pub fn install(info: AppInfo) -> bool {
    INFO.set(info).is_ok()
}

/// The installed names, or [`AppInfo::NEUTRAL`].
pub fn get() -> &'static AppInfo {
    INFO.get().unwrap_or(&AppInfo::NEUTRAL)
}

/// The short product name.
pub fn name() -> &'static str {
    get().name
}

/// The full product name.
pub fn full_name() -> &'static str {
    get().full_name
}

/// The stable name used by credential groups and terminal identification.
pub fn identity_name() -> &'static str {
    get().identity_name
}

/// The technical name.
pub fn slug() -> &'static str {
    get().slug
}

/// The environment variable prefix, without the trailing `_`.
pub fn env_prefix() -> &'static str {
    get().env_prefix
}

/// The product's folder name in a project root.
pub fn project_dir() -> &'static str {
    get().project_dir
}

/// The bundle identifier.
pub fn bundle_id() -> &'static str {
    get().bundle_id
}

/// The command-line program's name.
pub fn cli_name() -> &'static str {
    get().cli_name
}

/// The database command-line program's name.
pub fn db_cli_name() -> &'static str {
    get().db_cli_name
}

/// The programs the product ships.
pub fn binaries() -> &'static [&'static [&'static str]] {
    get().binaries
}

/// The name of the product's environment variable `suffix`:
/// `<prefix>_<suffix>`.
pub fn env_name(suffix: &str) -> String {
    format!("{}_{suffix}", env_prefix())
}

/// The value of the product's environment variable `suffix` (see
/// [`env_name`]); `None` when it is unset or not Unicode.
pub fn var(suffix: &str) -> Option<String> {
    var_os(suffix)?.into_string().ok()
}

/// Like [`var`], without requiring Unicode.
pub fn var_os(suffix: &str) -> Option<OsString> {
    get()
        .lookup_env(&env_name(suffix), |name| std::env::var_os(name))
        .map(|(_, value)| value)
}

fn profile_in(base: &Path) -> PathBuf {
    base.join(get().profile_dir)
}

/// Standard location for files such as the DB profiles and credential store
/// configuration. Those files have their own explicit path overrides.
pub fn default_config_dir_or_local() -> PathBuf {
    dirs::config_dir()
        .map(|base| profile_in(&base))
        .unwrap_or_else(|| profile_in(Path::new(".")))
}

/// The application's configuration folder: the environment variable
/// `<prefix>_CONFIG_DIR`, else the product profile in the user's configuration folder.
pub fn config_dir() -> Option<PathBuf> {
    match var_os("CONFIG_DIR").filter(|d| !d.is_empty()) {
        Some(dir) => Some(PathBuf::from(dir)),
        None => dirs::config_dir().map(|base| profile_in(&base)),
    }
}

/// The application's cache folder: the environment variable
/// `<prefix>_CACHE_DIR`, else the product profile in the user's cache folder
/// (`~/Library/Caches` on macOS, `$XDG_CACHE_HOME` or `~/.cache` on Linux,
/// the local application data folder on Windows).
pub fn cache_dir() -> Option<PathBuf> {
    match var_os("CACHE_DIR").filter(|d| !d.is_empty()) {
        Some(dir) => Some(PathBuf::from(dir)),
        None => dirs::cache_dir().map(|base| profile_in(&base)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_paths_are_lexical_and_use_only_the_product_names() {
        let base = Path::new("project-root");
        assert_eq!(
            AppInfo::NEUTRAL.project_storage_dir(base),
            base.join(".app")
        );
        assert_eq!(profile_in(base), base.join("app"));
    }

    #[test]
    fn env_names_join_prefix_and_suffix() {
        assert_eq!(env_name("CONFIG_DIR"), "APP_CONFIG_DIR");
    }

    #[test]
    fn the_neutral_names_are_the_default() {
        assert_eq!(AppInfo::default(), AppInfo::NEUTRAL);
        assert_eq!(slug(), "app");
        assert_eq!(project_dir(), ".app");
    }
}
