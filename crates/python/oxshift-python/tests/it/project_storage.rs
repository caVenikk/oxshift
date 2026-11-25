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
fn project_storage_only_uses_the_product_directory() {
    brand::install();
    let new = tempfile::tempdir().unwrap();
    read_only(new.path());
    assert!(!new.path().join(".oxshift").exists());
    write_state(new.path());
    assert!(new.path().join(".oxshift/project.toml").is_file());
    assert!(new.path().join(".oxshift/.gitignore").is_file());
    assert_eq!(std::fs::read_dir(new.path()).unwrap().count(), 1);

    for neighbor_is_file in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let neighbor = root.path().join(".other-tool");
        let original = "version = 1\n[exclusions]\npaths = [\"other-tool-only\"]\n";
        let sentinel = if neighbor_is_file {
            neighbor.clone()
        } else {
            std::fs::create_dir(&neighbor).unwrap();
            neighbor.join("project.toml")
        };
        std::fs::write(&sentinel, original).unwrap();
        read_only(root.path());
        assert!(!root.path().join(".oxshift").exists());
        write_state(root.path());
        assert_eq!(std::fs::read_to_string(&sentinel).unwrap(), original);
        assert!(root.path().join(".oxshift/project.toml").is_file());
        if !neighbor_is_file {
            assert_eq!(std::fs::read_dir(&neighbor).unwrap().count(), 1);
        }
        let filter = project::Filter::for_root(root.path());
        assert!(filter.is_hidden(".oxshift"));
        assert!(!filter.is_hidden(".other-tool"));
        assert!(!project::is_hidden_path(&sentinel, &filter));
        let changes = project::disk_changes(
            [
                (sentinel.clone(), false),
                (root.path().join(".oxshift/workspace.toml"), false),
            ],
            &filter,
        );
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path(), sentinel);
    }
}

#[test]
fn occupied_project_directory_reports_error_without_alternate_writes() {
    brand::install();
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join(".oxshift"), "not a directory").unwrap();
    assert!(store::load(root.path()).is_err());
    assert!(workspace::load_value(root.path(), "test", "value").is_err());
    assert!(workspace::save_value(root.path(), "test", "value", Some("x")).is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    assert_eq!(
        std::fs::read_to_string(root.path().join(".oxshift")).unwrap(),
        "not a directory"
    );
}

#[test]
fn idea_importers_preserve_unrelated_project_files() {
    brand::install();
    for neighbor_is_file in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let neighbor = root.path().join(".other-tool");
        let sentinel = if neighbor_is_file {
            neighbor.clone()
        } else {
            std::fs::create_dir(&neighbor).unwrap();
            neighbor.join("project.toml")
        };
        // Malformed settings from another tool must not block import.
        std::fs::write(&sentinel, "not valid TOML [").unwrap();
        let idea = root.path().join(".idea");
        std::fs::create_dir_all(idea.join("runConfigurations")).unwrap();
        let iml = r#"<module type="PYTHON_MODULE" version="4"><component name="NewModuleRootManager"><content url="file://$MODULE_DIR$"><excludeFolder url="file://$MODULE_DIR$/generated" /></content></component></module>"#;
        let modules = r#"<project version="4"><component name="ProjectModuleManager"><modules><module fileurl="file://$PROJECT_DIR$/.idea/demo.iml" filepath="$PROJECT_DIR$/.idea/demo.iml" /></modules></component></project>"#;
        let run = r#"<component name="ProjectRunConfigurationManager"><configuration name="from-idea" type="ShConfigurationType"><option name="SCRIPT_PATH" value="$PROJECT_DIR$/build.sh" /><method v="2" /></configuration></component>"#;
        std::fs::write(idea.join("modules.xml"), modules).unwrap();
        std::fs::write(idea.join("demo.iml"), iml).unwrap();
        std::fs::write(idea.join("runConfigurations/build.xml"), run).unwrap();
        let exclusions = store::import_idea(root.path()).unwrap().unwrap();
        assert!(exclusions.paths.contains("generated"));
        let imported = run_config::import::open(
            root.path(),
            &[&run_config::shell::ShellScriptImporter],
            &run_config::Types::default(),
        )
        .unwrap();
        assert!(imported.errors.is_empty());
        assert_eq!(imported.report.unwrap().imported, 1);
        assert_eq!(imported.list.all()[0].name, "from-idea");
        assert!(root.path().join(".oxshift/run/from-idea.toml").is_file());
        assert!(root.path().join(".oxshift/project.toml").is_file());
        assert_eq!(
            std::fs::read_to_string(&sentinel).unwrap(),
            "not valid TOML ["
        );
        if !neighbor_is_file {
            assert_eq!(std::fs::read_dir(&neighbor).unwrap().count(), 1);
        }
        assert_eq!(
            std::fs::read_to_string(idea.join("modules.xml")).unwrap(),
            modules
        );
        assert_eq!(std::fs::read_to_string(idea.join("demo.iml")).unwrap(), iml);
        assert_eq!(
            std::fs::read_to_string(idea.join("runConfigurations/build.xml")).unwrap(),
            run
        );
    }
}
