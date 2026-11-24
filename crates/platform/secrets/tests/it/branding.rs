use secrets::keepass::{KdfParams, KeePassKey, KeePassOptions, KeePassStore};
use secrets::{Secret, SecretStore};

#[test]
fn display_name_change_reuses_existing_keepass_group() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secrets.kdbx");
    let key = || KeePassKey::new(Secret::new("test-master"));
    // Create the pre-rebrand layout with the neutral product's original name.
    let old = KeePassStore::open(
        &path,
        key(),
        KeePassOptions {
            create: true,
            kdf: KdfParams {
                iterations: 1,
                memory_bytes: 64 * 1024,
                parallelism: 1,
            },
        },
    )
    .unwrap();
    old.set("app-db", "local", &Secret::new("before")).unwrap();
    drop(old);

    let config_dir = app_info::config_dir();
    let cache_dir = app_info::cache_dir();
    assert!(app_info::install(app_info::AppInfo {
        name: "Renamed App",
        full_name: "Renamed App IDE",
        legacy_env_prefix: Some("OLD"),
        ..app_info::AppInfo::NEUTRAL
    }));
    assert_eq!(app_info::config_dir(), config_dir);
    assert_eq!(app_info::cache_dir(), cache_dir);
    assert_eq!(app_info::slug(), "app");
    assert_eq!(app_info::db_cli_name(), "app-db");
    environment_aliases();
    let renamed = KeePassStore::open(&path, key(), KeePassOptions::default()).unwrap();
    assert_eq!(
        renamed.get("app-db", "local").unwrap().unwrap().expose(),
        "before"
    );
    renamed
        .set("app-db", "local", &Secret::new("after"))
        .unwrap();
    renamed
        .set("app-db", "second", &Secret::new("new"))
        .unwrap();
    drop(renamed);

    let reopened = KeePassStore::open(&path, key(), KeePassOptions::default()).unwrap();
    assert_eq!(
        reopened.get("app-db", "local").unwrap().unwrap().expose(),
        "after"
    );
    assert_eq!(
        reopened.list().unwrap(),
        vec![
            ("app-db".into(), "local".into()),
            ("app-db".into(), "second".into())
        ]
    );
    // Inspect the file as well: a renamed group must not duplicate the old one.
    let database = keepass::Database::parse(
        &std::fs::read(&path).unwrap(),
        keepass::DatabaseKey::new().with_password("test-master"),
    )
    .unwrap();
    let root = database.root();
    let groups: Vec<_> = root.groups().map(|g| g.name.clone()).collect();
    assert_eq!(groups, ["App"]);
    let group = root.groups().next().unwrap();
    let service = group.groups().next().unwrap();
    assert_eq!(service.entries().count(), 2);
}

fn environment_aliases() {
    use secrets::config::{
        KeePassConfig, MasterPasswordSource, config_path, keepass_master_password,
    };
    use std::sync::Arc;
    let store = secrets::MemoryStore::default();
    let mut config = KeePassConfig::new("unused.kdbx");
    config.master_password = MasterPasswordSource::Env;
    for (canonical, legacy, expected) in [
        (None, Some("old"), "old"),
        (Some("new"), Some("old"), "new"),
        (Some(""), Some("old"), ""),
    ] {
        let env: secrets::EnvLookup = Arc::new(move |name| match name {
            "APP_KEEPASS_PASSWORD" | "APP_SECRETS_CONFIG" => canonical.map(str::to_owned),
            "OLD_KEEPASS_PASSWORD" | "OLD_SECRETS_CONFIG" => legacy.map(str::to_owned),
            _ => None,
        });
        if expected.is_empty() {
            assert!(config_path(&env).ends_with("secrets.toml"));
            assert!(keepass_master_password(&config, None, &env, &store).is_err());
            continue;
        }
        assert_eq!(config_path(&env), std::path::PathBuf::from(expected));
        assert_eq!(
            keepass_master_password(&config, None, &env, &store)
                .unwrap()
                .expose(),
            expected
        );
    }
}
