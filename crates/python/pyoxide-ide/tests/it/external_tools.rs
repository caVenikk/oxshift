use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use gpui::{AppContext as _, Modifiers, TestAppContext, VisualTestContext, WindowHandle};
use serde_json::{Value, json};
use workbench::Workbench;

const PAGE: &str = "python.tools.external-tools";

struct H<'a> {
    cx: &'a mut TestAppContext,
    window: WindowHandle<Workbench>,
}

impl H<'_> {
    fn draw(&mut self) {
        self.cx.run_until_parked();
        self.cx
            .update_window(self.window.into(), |_, window, cx| window.draw(cx).clear())
            .unwrap();
    }

    fn try_invoke(&mut self, id: &str, args: Value) -> Result<Value, String> {
        let out = self
            .cx
            .update_window(self.window.into(), |_, window, cx| {
                ui::command::invoke(id, args, window, cx)
            })
            .unwrap()
            .map_err(|e| format!("{e:?}"));
        self.draw();
        out
    }

    fn invoke(&mut self, id: &str, args: Value) -> Value {
        self.try_invoke(id, args)
            .unwrap_or_else(|e| panic!("{id}: {e}"))
    }

    fn page(&mut self) -> Value {
        self.invoke("settings.state", Value::Null)["fields"].clone()
    }

    fn settings(&mut self) -> WindowHandle<settings_ui::SettingsWindow> {
        self.cx
            .update(|cx| settings_ui::find(cx))
            .expect("the Settings window")
    }

    fn bounds(&mut self, selector: &'static str) -> Option<gpui::Bounds<gpui::Pixels>> {
        self.draw();
        let settings = self.settings();
        VisualTestContext::from_window(settings.into(), self.cx).debug_bounds(selector)
    }

    fn click(&mut self, selector: &'static str) {
        let at = self
            .bounds(selector)
            .unwrap_or_else(|| panic!("{selector} is not drawn"))
            .center();
        let settings = self.settings();
        let vcx = &mut VisualTestContext::from_window(settings.into(), self.cx);
        vcx.simulate_mouse_move(at, None, Modifiers::none());
        vcx.simulate_click(at, Modifiers::none());
        self.draw();
    }

    /// Lets the writes and the search that wait for a pause run.
    fn settle(&mut self) {
        for _ in 0..50 {
            self.cx.executor().advance_clock(Duration::from_millis(150));
            self.draw();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Closes the Settings window, whatever the page's draft is.
    fn close_settings(&mut self) {
        self.cx.update(|cx| {
            if let Some(handle) = settings_ui::find(cx) {
                let _ = handle.update(cx, |_, window, _| window.remove_window());
            }
        });
    }
}

fn open<'a>(cx: &'a mut TestAppContext, root: &Path) -> H<'a> {
    cx.executor().allow_parking();
    let product = Rc::new(pyoxide_ide::product_with_tools(Default::default()));
    let window = cx.update(|cx| {
        workbench::init(cx, &product);
        workbench::open_main_window(cx, product.clone())
    });
    let mut h = H { cx, window };
    h.draw();
    h.invoke(
        "project.open-folder",
        json!({"path": root.display().to_string()}),
    );
    h
}

fn project() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = native_path::canonicalize(dir.path()).unwrap();
    std::fs::write(root.join("main.py"), "print('hi')\n").unwrap();
    (dir, root)
}

#[gpui::test]
fn lookup_mouse_selection_updates_applies_and_survives_reopen(cx: &mut TestAppContext) {
    lookup_roundtrip(cx, false, false);
}

#[gpui::test]
fn independent_lookup_mouse_selection_updates_applies_and_survives_reopen(cx: &mut TestAppContext) {
    let _surfaces = ui::popup_window::testing::independent_surfaces();
    lookup_roundtrip(cx, true, true);
}

#[gpui::test]
fn independent_lookup_choice_after_owner_activation(cx: &mut TestAppContext) {
    let _surfaces = ui::popup_window::testing::independent_surfaces();
    lookup_roundtrip(cx, true, false);
}

fn lookup_roundtrip(cx: &mut TestAppContext, independent: bool, delayed_activation: bool) {
    let (_dir, root) = project();
    let mut h = open(cx, &root);
    h.invoke("settings.open", json!({"page": PAGE}));
    h.settle();
    for (index, (tool, switch, lookup)) in [
        (
            "basedpyright",
            "external-tools:switch:basedpyright",
            "external-tools:lookup:basedpyright",
        ),
        (
            "ruff",
            "external-tools:switch:ruff",
            "external-tools:lookup:ruff",
        ),
        ("ty", "external-tools:switch:ty", "external-tools:lookup:ty"),
    ]
    .into_iter()
    .enumerate()
    {
        if h.page()["rows"][index]["enabled"] == false {
            h.click(switch);
            h.click("settings-apply");
            h.settle();
        }
        for (value, label) in [
            ("path", "menu-item:Path · uvx"),
            ("uvx", "menu-item:uvx"),
            ("sdk", "menu-item:Sdk · Path · uvx"),
        ] {
            let settings = h.settings();
            settings
                .update(h.cx, |_, window, _| window.activate_window())
                .unwrap();
            h.draw();
            h.click(lookup);
            assert_eq!(h.page()["list"], tool);
            let popup =
                h.cx.update(|cx| ui::popup_window::innermost(settings.into(), cx));
            assert_eq!(popup != settings.into(), independent);
            if delayed_activation {
                // AppKit may deliver the owner's return activation after the
                // choice. Keep activation under test control until it is clicked.
                VisualTestContext::from_window(popup, h.cx).deactivate_window();
                h.draw();
                assert!(
                    !h.cx
                        .update_window(settings.into(), |_, window, _| window.is_window_active())
                        .unwrap()
                );
            }
            {
                let vcx = &mut VisualTestContext::from_window(popup, h.cx);
                vcx.update(|window, cx| window.draw(cx).clear());
                let at = vcx.debug_bounds(label).unwrap().center();
                vcx.simulate_mouse_move(at, None, Modifiers::none());
                vcx.simulate_click(at, Modifiers::none());
            }
            h.draw();
            assert_eq!(h.page()["rows"][index]["lookup"], value);
            if independent {
                settings
                    .update(h.cx, |_, window, _| window.activate_window())
                    .unwrap();
                h.draw();
            }
            assert_eq!(h.page()["rows"][index]["lookup"], value);
            assert_eq!(
                h.invoke("settings.state", Value::Null)["apply_enabled"],
                true
            );
            h.click("settings-apply");
            h.settle();
            assert_eq!(
                py_lsp::settings::load(&root)
                    .unwrap()
                    .tool(py_lsp::settings::Tool::parse(tool).unwrap())
                    .lookup
                    .id(),
                value
            );
            assert_eq!(
                h.invoke("settings.state", Value::Null)["apply_enabled"],
                false
            );
            h.click(lookup);
            let popup =
                h.cx.update(|cx| ui::popup_window::innermost(settings.into(), cx));
            h.cx.simulate_keystrokes(popup, "down escape");
            h.draw();
            assert_eq!(h.page()["list"], Value::Null);
            assert_eq!(h.page()["rows"][index]["lookup"], value);
            assert_eq!(
                h.invoke("settings.state", Value::Null)["apply_enabled"],
                false
            );
            h.close_settings();
            h.invoke("settings.open", json!({"page": PAGE}));
            h.settle();
            assert_eq!(h.page()["rows"][index]["lookup"], value);
        }
    }
    h.close_settings();
}
