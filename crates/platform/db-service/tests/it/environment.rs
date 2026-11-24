use std::sync::Arc;

use db_core::Backend;
use db_service::profiles::{Profile, TlsProfile};
use db_service::secrets::{SecretKind, SecretResolver, SecretSource};
use secrets::{EnvLookup, MemoryStore, Secret, SecretStore};

#[test]
fn canonical_secret_presence_wins_and_diagnostics_name_the_source() {
    assert!(app_info::install(app_info::AppInfo {
        db_cli_name: "new-db",
        env_prefix: "NEW",
        legacy_env_prefix: Some("OLD"),
        ..app_info::AppInfo::NEUTRAL
    }));
    let store = MemoryStore::default();
    for account in ["local", "local/wallet"] {
        store.set("app-db", account, &Secret::new("saved")).unwrap();
    }
    let mut profile = Profile::new(Backend::Postgres);
    profile.tls = Some(TlsProfile {
        wallet_dir: Some("unused-wallet".into()),
        ..TlsProfile::default()
    });
    for kind in [SecretKind::Password, SecretKind::WalletPassword] {
        let suffix = match kind {
            SecretKind::Password => "DB_PASSWORD",
            SecretKind::WalletPassword => "DB_WALLET_PASSWORD",
        };
        for (canonical, legacy, value, prefix) in [
            (None, Some("old"), "old", "OLD"),
            (Some("new"), Some("old"), "new", "NEW"),
            (Some(""), Some("old"), "saved", ""),
            (None, Some(""), "saved", ""),
        ] {
            let env: EnvLookup = Arc::new(move |name| {
                if name == format!("NEW_{suffix}") {
                    canonical.map(str::to_owned)
                } else if name == format!("OLD_{suffix}") {
                    legacy.map(str::to_owned)
                } else {
                    None
                }
            });
            let resolver = SecretResolver {
                store: &store,
                prompter: None,
                env: &env,
            };
            let (secret, source, error) = resolver.resolve("local", &profile, kind);
            assert_eq!(secret.expose(), value);
            let expected = if prefix.is_empty() {
                SecretSource::Store(store.name().to_owned())
            } else {
                SecretSource::Env(format!("{prefix}_{suffix}"))
            };
            assert_eq!(source, expected);
            assert!(error.is_none());
        }
    }
    // A profile may explicitly contain the old generic name. It still resolves
    // canonically, while arbitrary user-chosen names retain their precedence.
    profile.password_env = Some("OLD_DB_PASSWORD".into());
    let env: EnvLookup = Arc::new(|name| match name {
        "NEW_DB_PASSWORD" => Some("canonical".into()),
        "OLD_DB_PASSWORD" => Some("legacy".into()),
        "CUSTOM_PASSWORD" => Some("custom".into()),
        _ => None,
    });
    let resolver = SecretResolver {
        store: &store,
        prompter: None,
        env: &env,
    };
    assert_eq!(
        resolver
            .resolve("local", &profile, SecretKind::Password)
            .0
            .expose(),
        "canonical"
    );
    profile.password_env = Some("CUSTOM_PASSWORD".into());
    assert_eq!(
        resolver
            .resolve("local", &profile, SecretKind::Password)
            .0
            .expose(),
        "custom"
    );
}
