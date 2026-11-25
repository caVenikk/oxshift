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
        ..app_info::AppInfo::NEUTRAL
    }));
    assert_eq!(app_info::config_dir(), config_dir);
    assert_eq!(app_info::cache_dir(), cache_dir);
    assert_eq!(app_info::slug(), "app");
    assert_eq!(app_info::db_cli_name(), "app-db");
    environment_values();
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

fn environment_values() {
    use secrets::config::{
        KeePassConfig, MasterPasswordSource, config_path, keepass_master_password,
    };
    use std::sync::Arc;
    let store = secrets::MemoryStore::default();
    let mut config = KeePassConfig::new("unused.kdbx");
    config.master_password = MasterPasswordSource::Env;
    for value in [None, Some("configured"), Some("")] {
        let env: secrets::EnvLookup = Arc::new(move |name| match name {
            "APP_KEEPASS_PASSWORD" | "APP_SECRETS_CONFIG" => value.map(str::to_owned),
            _ => None,
        });
        match value.filter(|value| !value.is_empty()) {
            Some(expected) => {
                assert_eq!(config_path(&env), std::path::PathBuf::from(expected));
                assert_eq!(
                    keepass_master_password(&config, None, &env, &store)
                        .unwrap()
                        .expose(),
                    expected
                );
            }
            None => {
                assert!(config_path(&env).ends_with("secrets.toml"));
                assert!(keepass_master_password(&config, None, &env, &store).is_err());
            }
        }
    }
}

#[test]
fn new_identity_uses_a_separate_group_without_removing_saved_passwords() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secrets.kdbx");
    let key = || KeePassKey::new(Secret::new("test-master"));
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
    old.set("app-db", "local", &Secret::new("old-password"))
        .unwrap();
    drop(old);
    assert!(app_info::install(app_info::AppInfo {
        name: "New App",
        identity_name: "New App",
        slug: "new-app",
        ..app_info::AppInfo::NEUTRAL
    }));
    let new = KeePassStore::open(&path, key(), KeePassOptions::default()).unwrap();
    assert!(new.get("app-db", "local").unwrap().is_none());
    assert!(new.get("new-app-db", "local").unwrap().is_none());
    new.set("new-app-db", "local", &Secret::new("new-password"))
        .unwrap();
    assert_eq!(new.list().unwrap(), [("new-app-db".into(), "local".into())]);
    let database = keepass::Database::parse(
        &std::fs::read(&path).unwrap(),
        keepass::DatabaseKey::new().with_password("test-master"),
    )
    .unwrap();
    let root = database.root();
    let groups: Vec<_> = root.groups().map(|group| group.name.clone()).collect();
    assert_eq!(groups, ["App", "New App"]);
    let old_group = root.groups().find(|group| group.name == "App").unwrap();
    let old_service = old_group.groups().next().unwrap();
    assert_eq!(old_service.entries().count(), 1);
    assert_eq!(
        old_service.entries().next().unwrap().get_password(),
        Some("old-password")
    );
}
