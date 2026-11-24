//! The product's name in one place (docs/rebrand.md).
//!
//! Every other crate of the product layer takes the name from here, and the
//! platform gets it at startup through [`install`] (`app-info`). The
//! values below are what existing users have on disk (settings and cache
//! folders, keychain items, the project folder, environment variables), so
//! changing one is a migration, not an edit. `scripts/check-brand.py` fails
//! on the name anywhere else in the code.

use app_info::AppInfo;

macro_rules! slug {
    () => {
        "pyoxide"
    };
}

/// The prefix of the environment variables, as a literal for `concat!`.
#[doc(hidden)]
#[macro_export]
macro_rules! env_prefix {
    () => {
        "OXSHIFT"
    };
}

/// Name of an environment variable of the product built at compile time:
/// `brand::env_name!("UV")` is `"<PREFIX>_UV"`. For `concat!` and
/// attributes; at run time `app_info::env_name` does the same.
#[macro_export]
macro_rules! env_name {
    ($suffix:literal) => {
        concat!($crate::env_prefix!(), "_", $suffix)
    };
}

/// Short name, in sentences and menus.
pub const PRODUCT_NAME: &str = "Oxshift Python";

/// Full name, in window titles and user agents.
pub const PRODUCT_FULL_NAME: &str = "Oxshift Python IDE";

/// Stable, case-sensitive identity used by existing KeePass groups and shells.
/// Unlike display names, changing this requires a compatibility migration.
pub const IDENTITY_NAME: &str = "Pyoxide";

/// Stable technical name for database sessions and the keychain service prefix.
pub const SLUG: &str = slug!();

/// Prefix of the environment variables (`<ENV_PREFIX>_CONFIG_DIR`, ...).
pub const ENV_PREFIX: &str = env_prefix!();

/// Compatibility input prefix; canonical variables always take precedence.
pub const LEGACY_ENV_PREFIX: &str = "PYOXIDE";

/// Stable measurement output protocol, independent of environment names.
pub const MEASUREMENT_PREFIX: &str = "PYOXIDE";

/// Directory for new user profiles and caches under the OS standard locations.
pub const PROFILE_DIR: &str = "oxshift-python";

/// The product's folder in a project root.
pub const PROJECT_DIR: &str = ".oxshift";

/// Existing project stores remain in place until explicitly migrated.
pub const LEGACY_PROJECT_DIR: &str = ".oxide";

/// Identifier of the application bundle.
pub const BUNDLE_ID: &str = "dev.pyoxide.ide";

/// The command-line program.
pub const CLI_NAME: &str = "oxshift";

/// The database command-line program.
pub const DB_CLI_NAME: &str = "oxshift-db";

/// The IDE program.
pub const IDE_NAME: &str = "oxshift-python";

/// The product's names as the platform takes them.
pub const INFO: AppInfo = AppInfo {
    name: PRODUCT_NAME,
    full_name: PRODUCT_FULL_NAME,
    identity_name: IDENTITY_NAME,
    slug: SLUG,
    profile_dir: PROFILE_DIR,
    legacy_profile_dir: Some(SLUG),
    env_prefix: ENV_PREFIX,
    legacy_env_prefix: Some(LEGACY_ENV_PREFIX),
    measurement_prefix: MEASUREMENT_PREFIX,
    project_dir: PROJECT_DIR,
    legacy_project_dir: Some(LEGACY_PROJECT_DIR),
    bundle_id: BUNDLE_ID,
    cli_name: CLI_NAME,
    db_cli_name: DB_CLI_NAME,
    binaries: &[
        &[IDE_NAME, concat!(slug!(), "-ide")],
        &[DB_CLI_NAME, concat!(slug!(), "-db")],
        &[CLI_NAME, slug!()],
    ],
};

/// Read a product environment variable even before platform installation.
/// Canonical presence wins over the compatibility name, including empty values.
pub fn var_os(suffix: &str) -> Option<std::ffi::OsString> {
    INFO.lookup_env(&format!("{ENV_PREFIX}_{suffix}"), |name| {
        std::env::var_os(name)
    })
    .map(|(_, value)| value)
}

/// Read a Unicode product environment variable.
pub fn var(suffix: &str) -> Option<String> {
    var_os(suffix)?.into_string().ok()
}

/// Tells the platform the product's names. Call first in `main`, before
/// anything reads a setting.
pub fn install() {
    app_info::install(INFO);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_values_stay_what_existing_users_have_on_disk() {
        assert_eq!(IDENTITY_NAME, "Pyoxide");
        assert_eq!(BUNDLE_ID, "dev.pyoxide.ide");
        assert_eq!(CLI_NAME, "oxshift");
        assert_eq!(SLUG, "pyoxide");
        assert_eq!(INFO.profile_dir, "oxshift-python");
        assert_eq!(INFO.legacy_profile_dir, Some("pyoxide"));
        assert_eq!(ENV_PREFIX, "OXSHIFT");
        assert_eq!(LEGACY_ENV_PREFIX, "PYOXIDE");
        assert_eq!(MEASUREMENT_PREFIX, "PYOXIDE");
        assert_eq!(PROJECT_DIR, ".oxshift");
        assert_eq!(LEGACY_PROJECT_DIR, ".oxide");
        assert_eq!(DB_CLI_NAME, "oxshift-db");
        assert_eq!(IDE_NAME, "oxshift-python");
        assert_eq!(INFO.binaries[0], ["oxshift-python", "pyoxide-ide"]);
        assert_eq!(INFO.binaries[1], ["oxshift-db", "pyoxide-db"]);
        assert_eq!(INFO.binaries[2], ["oxshift", "pyoxide"]);
        assert_eq!(env_name!("CONFIG_DIR"), "OXSHIFT_CONFIG_DIR");
    }

    #[test]
    fn display_names_are_independent_of_the_persistent_identity() {
        assert_eq!(PRODUCT_NAME, "Oxshift Python");
        assert_eq!(PRODUCT_FULL_NAME, "Oxshift Python IDE");
        assert_eq!(INFO.identity_name, "Pyoxide");
        assert_ne!(INFO.name, INFO.identity_name);
    }

    #[test]
    fn canonical_environment_presence_wins_over_legacy() {
        for (canonical, legacy, expected) in [
            (None, None, None),
            (None, Some("old"), Some(("PYOXIDE_UV", "old"))),
            (Some("new"), None, Some(("OXSHIFT_UV", "new"))),
            (Some("new"), Some("old"), Some(("OXSHIFT_UV", "new"))),
            (Some(""), Some("old"), Some(("OXSHIFT_UV", ""))),
            (None, Some(""), Some(("PYOXIDE_UV", ""))),
        ] {
            for requested in ["OXSHIFT_UV", "PYOXIDE_UV"] {
                let actual = INFO.lookup_env(requested, |name| match name {
                    "OXSHIFT_UV" => canonical,
                    "PYOXIDE_UV" => legacy,
                    _ => None,
                });
                assert_eq!(
                    actual.as_ref().map(|(name, value)| (name.as_str(), *value)),
                    expected
                );
            }
        }
        assert_eq!(
            INFO.lookup_env("CUSTOM_PASSWORD", |name| Some(name.to_owned())),
            Some(("CUSTOM_PASSWORD".into(), "CUSTOM_PASSWORD".into()))
        );
    }

    #[test]
    fn install_gives_the_platform_the_names() {
        install();
        assert_eq!(app_info::env_name("UV"), env_name!("UV"));
        assert_eq!(app_info::slug(), SLUG);
        assert_eq!(app_info::name(), PRODUCT_NAME);
        assert_eq!(app_info::full_name(), PRODUCT_FULL_NAME);
        assert_eq!(app_info::identity_name(), IDENTITY_NAME);
    }
}
