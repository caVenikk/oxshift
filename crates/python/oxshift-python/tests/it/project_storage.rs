use std::path::Path;

use project::{store, workspace};
use run_config::{RunConfiguration, RunConfigurations, Storage};

fn read_only(root: &Path) {
    assert!(store::load(root).unwrap().is_none());
    assert!(
        workspace::load_value(root, "test", "value")
            .unwrap()
            .is_none()
    );
    let runs = run_config::store::load(root).unwrap();
    assert!(runs.errors.is_empty());
    assert!(runs.list.all().is_empty());
}

fn write_state(root: &Path) {
    let mut exclusions = project::Exclusions::default();
    exclusions.add("build");
    store::save(root, &exclusions).unwrap();
    workspace::save_value(root, "test", "value", Some("saved")).unwrap();
    let mut runs = RunConfigurations::default();
    runs.add(RunConfiguration::new("python", "shared", Storage::Shared));
    runs.add(RunConfiguration::new(
        "python",
        "personal",
        Storage::Personal,
    ));
    run_config::store::save(root, &mut runs).unwrap();
    assert_eq!(
        workspace::load_value(root, "test", "value")
            .unwrap()
            .as_deref(),
        Some("saved")
    );
    assert!(
        run_config::store::shared_dir(root)
            .join("shared.toml")
            .is_file()
    );
}

#[test]
fn project_storage_upgrade_selects_one_complete_directory() {
    brand::install();
    let new = tempfile::tempdir().unwrap();
    read_only(new.path());
    assert!(!new.path().join(".oxshift").exists());
    assert!(!new.path().join(".oxide").exists());
    write_state(new.path());
    assert!(new.path().join(".oxshift/project.toml").is_file());
    assert!(new.path().join(".oxshift/.gitignore").is_file());
    assert!(!new.path().join(".oxide").exists());

    let old = tempfile::tempdir().unwrap();
    std::fs::create_dir(old.path().join(".oxide")).unwrap();
    read_only(old.path());
    assert_eq!(
        std::fs::read_dir(old.path().join(".oxide"))
            .unwrap()
            .count(),
        0
    );
    write_state(old.path());
    assert!(old.path().join(".oxide/project.toml").is_file());
    assert!(old.path().join(".oxide/.gitignore").is_file());
    assert!(!old.path().join(".oxshift").exists());
    let original = std::fs::read(old.path().join(".oxide/workspace.toml")).unwrap();
    let old_runs = std::fs::read(old.path().join(".oxide/run/shared.toml")).unwrap();

    // A canonical folder selects the entire store, even with no settings yet.
    std::fs::create_dir(old.path().join(".oxshift")).unwrap();
    read_only(old.path());
    write_state(old.path());
    assert_eq!(
        std::fs::read(old.path().join(".oxide/workspace.toml")).unwrap(),
        original
    );
    assert_eq!(
        std::fs::read(old.path().join(".oxide/run/shared.toml")).unwrap(),
        old_runs
    );
    let filter = project::Filter::for_root(old.path());
    for name in [".oxshift", ".oxide"] {
        assert!(filter.is_hidden(name));
        assert!(project::is_hidden_path(
            &old.path().join(name).join("project.toml"),
            &filter
        ));
    }
    assert!(
        project::disk_changes(
            [
                (old.path().join(".oxide/project.toml"), false),
                (old.path().join(".oxshift/workspace.toml"), false),
            ],
            &filter
        )
        .is_empty()
    );

    for blocked in [".oxshift", ".oxide"] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(blocked), "not a directory").unwrap();
        assert!(store::load(root.path()).is_err());
        assert!(workspace::save_value(root.path(), "test", "value", Some("x")).is_err());
        let other = if blocked == ".oxshift" {
            ".oxide"
        } else {
            ".oxshift"
        };
        assert!(!root.path().join(other).exists());
        assert_eq!(
            std::fs::read_to_string(root.path().join(blocked)).unwrap(),
            "not a directory"
        );
    }
}
