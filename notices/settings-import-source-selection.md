- crates/platform/settings-ui/src/first_run/view.rs
  Source selection follows the public IntelliJ Platform Community action flow
  (Apache-2.0, LICENSE-APACHE): one source activates it directly; several open
  a chooser, independent of mouse or keyboard activation. Reimplemented in Rust
  using the existing menu; no copied code or additional assets.
  Source: https://github.com/JetBrains/intellij-community/tree/pycharm/2026.2.3/plugins/ide-startup/importSettings/src/com/intellij/ide/startup/importSettings/chooser/productChooser
  Commit: 404a109fe630a2b9013f10b71081f87c9dbb32e2.
  Files: MainChooserAction.kt and ProductChooserAction.kt.
