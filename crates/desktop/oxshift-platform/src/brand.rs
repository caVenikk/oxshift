use app_info::AppInfo;

/// Standalone desktop identity, independent of language product profiles.
pub const INFO: AppInfo = AppInfo {
    name: "Oxshift",
    full_name: "Oxshift",
    identity_name: "Oxshift",
    slug: "oxshift-platform",
    profile_dir: "oxshift-platform",
    env_prefix: "OXSHIFT",
    project_dir: ".oxshift",
    bundle_id: "dev.oxshift.platform",
    cli_name: "oxshift",
    db_cli_name: "oxshift-db",
    binaries: &["oxshift-platform"],
};

pub const ICON: &[u8] = include_bytes!("../../../../assets/brand/oxshift.svg");
