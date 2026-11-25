//! The product's name in one place (docs/rebrand.md).
//!
//! Every other crate of the product layer takes the name from here, and the
//! platform gets it at startup through [`install`] (`app-info`). The
//! values below define the canonical runtime and storage namespaces. Retired
//! credential namespaces are not read or migrated automatically. `scripts/check-brand.py` fails
//! on the name anywhere else in the code.

use app_info::AppInfo;

macro_rules! slug {
    () => {
        "oxshift"
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

/// Case-sensitive group for this product in a KeePass database.
/// Retired groups are left untouched; passwords can be entered again.
pub const IDENTITY_NAME: &str = "Oxshift";

/// Technical name for database sessions, threads and new theme identifiers.
pub const SLUG: &str = slug!();

/// Prefix of the environment variables (`<ENV_PREFIX>_CONFIG_DIR`, ...).
pub const ENV_PREFIX: &str = env_prefix!();

/// Directory for user profiles and caches under the OS standard locations.
pub const PROFILE_DIR: &str = "oxshift-python";

/// The product's folder in a project root.
pub const PROJECT_DIR: &str = ".oxshift";

/// Identifier of the application bundle.
pub const BUNDLE_ID: &str = "dev.oxshift.python";

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
    env_prefix: ENV_PREFIX,
    project_dir: PROJECT_DIR,
    bundle_id: BUNDLE_ID,
    cli_name: CLI_NAME,
    db_cli_name: DB_CLI_NAME,
    binaries: &[IDE_NAME, DB_CLI_NAME, CLI_NAME],
};

/// Read a product environment variable even before platform installation.
/// Empty values remain present.
pub fn var_os(suffix: &str) -> Option<std::ffi::OsString> {
    std::env::var_os(format!("{ENV_PREFIX}_{suffix}"))
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
    fn canonical_interfaces_use_the_product_identity() {
        assert_eq!(IDENTITY_NAME, "Oxshift");
        assert_eq!(BUNDLE_ID, "dev.oxshift.python");
        assert_eq!(CLI_NAME, "oxshift");
        assert_eq!(SLUG, "oxshift");
        assert_eq!(INFO.profile_dir, "oxshift-python");
        assert_eq!(ENV_PREFIX, "OXSHIFT");
        assert_eq!(PROJECT_DIR, ".oxshift");
        assert_eq!(DB_CLI_NAME, "oxshift-db");
        assert_eq!(IDE_NAME, "oxshift-python");
        assert_eq!(INFO.binaries[0], "oxshift-python");
        assert_eq!(INFO.binaries[1], "oxshift-db");
        assert_eq!(INFO.binaries[2], "oxshift");
        assert_eq!(env_name!("CONFIG_DIR"), "OXSHIFT_CONFIG_DIR");
    }

    #[test]
    fn display_names_are_independent_of_the_persistent_identity() {
        assert_eq!(PRODUCT_NAME, "Oxshift Python");
        assert_eq!(PRODUCT_FULL_NAME, "Oxshift Python IDE");
        assert_eq!(INFO.identity_name, "Oxshift");
        assert_ne!(INFO.name, INFO.identity_name);
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
