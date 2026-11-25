//! The project's virtual environment in real shells
//! (tests/scenarios/terminal/open-and-venv.md, steps 1 and 2): a session
//! planned with the Python product's customizer in a project with a uv
//! environment shows the environment's prompt prefix and runs its Python,
//! in a shell with Oxshift's integration (zsh: the `activate` script is
//! sourced) and in one without (tcsh: the variables are merged).
//!
//! Ignored: they need zsh, tcsh and uv (macOS has the shells); run them with
//! `cargo test -p oxshift-python --test it terminal_venv:: -- --ignored`.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use gpui::AppContext as _;

use oxshift_python::terminal::VirtualEnvCustomizer;
use terminal::{Session, SessionEvent, StartRequest, TerminalSize, plan_session};

const TIMEOUT: Duration = Duration::from_secs(30);

/// A project with a uv environment `.venv`, and a home folder with empty
/// startup files.
struct Project {
    _dir: tempfile::TempDir,
    root: PathBuf,
    home: PathBuf,
}

fn project() -> Project {
    let dir = tempfile::tempdir().unwrap();
    let base = native_path::canonicalize(dir.path()).unwrap();
    let root = base.join("demo");
    let home = base.join("home");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join(".zshrc"), "").unwrap();
    std::fs::write(home.join(".tcshrc"), "set prompt = 'tcsh> '\n").unwrap();
    let status = Command::new("uv")
        .args(["venv", "--quiet", ".venv"])
        .current_dir(&root)
        .status()
        .expect("uv is installed");
    assert!(status.success());
    Project {
        _dir: dir,
        root,
        home,
    }
}

/// Starts `shell` in the project, types `line` once the screen shows
/// `prompt` and returns the screen with the result line.
fn run(project: &Project, shell: &str, prompt: &str, line: &str) -> String {
    assert!(Path::new(shell).is_file(), "{shell} is not installed");
    let scripts = tempfile::tempdir().unwrap();
    let home = project.home.to_string_lossy().into_owned();
    let request = StartRequest {
        shell_path: Some(shell.to_owned()),
        project_root: Some(project.root.clone()),
        project_env: vec![("HOME".into(), home.clone()), ("ZDOTDIR".into(), home)],
        size: TerminalSize::new(200, 40),
        integration_dir: Some(scripts.path().to_owned()),
        ..StartRequest::default()
    };
    let plan = plan_session(&request, &[&VirtualEnvCustomizer::default()]);
    let (session, rx) = Session::spawn(plan.options).expect("start the shell");
    let deadline = Instant::now() + TIMEOUT;
    let mut typed = false;
    let mut text = String::new();
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(SessionEvent::Exited(_)) => break,
            Ok(_) | Err(flume::RecvTimeoutError::Timeout) => {}
            Err(flume::RecvTimeoutError::Disconnected) => break,
        }
        text = session.snapshot().text();
        if !typed && text.contains(prompt) {
            session.write_text(&format!("{line}\r"));
            typed = true;
        } else if typed && result(&text).is_some() {
            break;
        }
    }
    session.kill();
    text
}

/// The result line `R:<VIRTUAL_ENV>:<python>:E` (not the command line).
fn result(text: &str) -> Option<String> {
    text.lines()
        .filter(|l| l.starts_with("R:") && l.trim_end().ends_with(":E"))
        .map(|l| l.trim_end().to_owned())
        .next_back()
}

fn expected(project: &Project) -> String {
    let venv = project.root.join(".venv");
    format!(
        "R:{}:{}:E",
        venv.display(),
        venv.join("bin").join("python").display()
    )
}

#[test]
#[ignore = "needs zsh and uv"]
fn zsh_sources_the_activation() {
    let project = project();
    let line = "echo \"R:$VIRTUAL_ENV:$(command -v python):E\"";
    let text = run(&project, "/bin/zsh", "(.venv) ", line);
    assert_eq!(result(&text), Some(expected(&project)), "{text}");
}

#[test]
#[ignore = "needs tcsh and uv"]
fn tcsh_gets_the_variables() {
    let project = project();
    // tcsh has no integration: the variables of the activation are merged,
    // the prompt prefix comes in PS1, which tcsh does not use.
    let line = "echo \"R:${VIRTUAL_ENV}:`which python`:E\"";
    let text = run(&project, "/bin/tcsh", "tcsh>", line);
    assert_eq!(result(&text), Some(expected(&project)), "{text}");
}

/// Exercises the product hook through the same SDK-selection and terminal
/// commands as the UI, with real environments (including symlinked Python).
#[gpui::test]
#[ignore = "needs zsh, tcsh and uv"]
fn new_tabs_follow_the_selected_external_sdk(cx: &mut gpui::TestAppContext) {
    check_selected_external_sdk(cx, "python");
}

#[gpui::test]
#[ignore = "needs zsh, tcsh and uv"]
fn new_tabs_follow_an_external_sdk_selected_by_python3(cx: &mut gpui::TestAppContext) {
    check_selected_external_sdk(cx, "python3");
}

fn check_selected_external_sdk(cx: &mut gpui::TestAppContext, executable: &str) {
    use std::rc::Rc;

    use serde_json::json;

    cx.executor().allow_parking();
    let a = project();
    let b = project();
    for p in [&a, &b] {
        std::fs::write(p.root.join("main.py"), "").unwrap();
        std::fs::write(
            p.root.join("probe.py"),
            "import json, os, shutil, sys\nfrom pathlib import Path\nPath(sys.argv[1]).write_text(json.dumps(dict(venv=os.environ.get('VIRTUAL_ENV'), python=shutil.which('python'), executable=sys.executable, cwd=os.getcwd())))\n",
        )
        .unwrap();
        project::workspace::save_toml_values(
            &p.root,
            "terminal.settings",
            vec![(
                "environment",
                Some(
                    [
                        format!("HOME={}", p.home.display()),
                        format!("ZDOTDIR={}", p.home.display()),
                    ]
                    .into_iter()
                    .collect(),
                ),
            )],
        )
        .unwrap();
    }
    let product = Rc::new(workbench::Product {
        code_intel: Vec::new(),
        ..oxshift_python::product_with_tools(Default::default())
    });
    let window = cx.update(|cx| {
        workbench::init(cx, &product);
        workbench::open_main_window(cx, product.clone())
    });
    let mut h = ProductTerminal { cx, window };
    h.open_project(&b.root);
    h.invoke(
        "python.interpreter.select",
        json!({"path": b.root.join(".venv/bin").join(executable)}),
    );
    // Selection registers B in the application SDK table, associated with B.
    let task =
        h.cx.update(|cx| oxshift_python::interpreter::for_project(&b.root, cx));
    h.cx.run_until_parked();
    let sdk = h.cx.executor().block_test(task).unwrap();
    h.open_project(&a.root);
    h.invoke("python.interpreter.select", json!({"name": sdk.name}));
    h.probe(&a.root, "/bin/zsh", Some(&b.root), "external");
    h.probe(&a.root, "/bin/tcsh", Some(&b.root), "fallback");

    h.invoke(
        "python.interpreter.select",
        json!({"path": a.root.join(".venv/bin/python")}),
    );
    h.probe(&a.root, "/bin/zsh", Some(&a.root), "changed");
    // Changing A does not change B's selection.
    h.open_project(&b.root);
    h.probe(&b.root, "/bin/zsh", Some(&b.root), "isolated");
    h.open_project(&a.root);
    h.invoke("python.interpreter.select", json!({"name": sdk.name}));
    oxshift_python::terminal::set_activate_virtualenv(&a.root, false).unwrap();
    for (shell, name) in [("/bin/zsh", "disabled"), ("/bin/tcsh", "disabled-fallback")] {
        let observed = h.probe(&a.root, shell, None, name);
        // An unrelated environment inherited from the parent is valid;
        // neither the selected SDK nor the local environment may be injected.
        for root in [&a.root, &b.root] {
            let env = root.join(".venv");
            assert_ne!(observed["venv"], json!(env), "{name}: {observed}");
            for key in ["python", "executable"] {
                assert!(
                    observed[key]
                        .as_str()
                        .is_none_or(|p| !Path::new(p).starts_with(&env)),
                    "{name}: {observed}"
                );
            }
        }
    }
}

struct ProductTerminal<'a> {
    cx: &'a mut gpui::TestAppContext,
    window: gpui::WindowHandle<workbench::Workbench>,
}

impl ProductTerminal<'_> {
    fn invoke(&mut self, id: &str, args: serde_json::Value) -> serde_json::Value {
        let result = self
            .cx
            .update_window(self.window.into(), |_, window, cx| {
                ui::command::invoke(id, args, window, cx)
            })
            .unwrap()
            .unwrap_or_else(|e| panic!("{id}: {e:?}"));
        self.cx.run_until_parked();
        result
    }

    fn tick(&mut self) {
        self.cx.executor().advance_clock(Duration::from_millis(10));
        self.cx.run_until_parked();
        self.cx
            .update_window(self.window.into(), |_, window, cx| window.draw(cx).clear())
            .unwrap();
        std::thread::sleep(Duration::from_millis(10));
    }

    fn open_project(&mut self, root: &Path) {
        self.invoke("project.open-folder", serde_json::json!({"path": root}));
        self.invoke("project.open-file", serde_json::json!({"path": "main.py"}));
        self.cx.run_until_parked();
    }

    fn probe(
        &mut self,
        root: &Path,
        shell: &str,
        expected: Option<&Path>,
        name: &str,
    ) -> serde_json::Value {
        use serde_json::json;
        let before = self.invoke("terminal.state", json!(null))["tabs"]
            .as_array()
            .map_or(0, Vec::len);
        self.invoke("terminal.open", json!({"shell": shell}));
        if before != 0 {
            self.invoke("terminal.new-tab", json!({"shell": shell}));
        }
        let deadline = Instant::now() + TIMEOUT;
        loop {
            self.tick();
            let state = self.invoke("terminal.state", json!(null));
            if state["tabs"]
                .as_array()
                .is_some_and(|t| t.len() == before + 1)
                && state["started"] == true
                && state["lines"]
                    .as_array()
                    .is_some_and(|l| l.iter().any(|v| !v.as_str().unwrap_or_default().is_empty()))
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "{name}: shell did not start: {state}"
            );
        }
        let output = root.join(format!("{name}.json"));
        let python = if expected.is_some() {
            "python"
        } else {
            "python3"
        };
        self.invoke(
            "terminal.type",
            json!({"text": format!("{python} probe.py {name}.json\r")}),
        );
        let observed: serde_json::Value = loop {
            self.tick();
            if let Ok(text) = std::fs::read_to_string(&output)
                && let Ok(value) = serde_json::from_str(&text)
            {
                break value;
            }
            assert!(
                Instant::now() < deadline,
                "{name}: probe did not finish: {}",
                self.invoke("terminal.state", json!(null))
            );
        };
        assert_eq!(observed["cwd"], json!(root), "{name}");
        if let Some(expected) = expected {
            assert_eq!(observed["venv"], json!(expected.join(".venv")), "{name}");
            assert_eq!(
                observed["python"],
                json!(expected.join(".venv/bin/python")),
                "{name}"
            );
            assert_eq!(observed["executable"], observed["python"], "{name}");
        }
        observed
    }
}
