- crates/platform/vcs-ui/src/update.rs, crates/platform/workbench/src/workbench/remote.rs
  and crates/platform/git/src/remote.rs: the Update Project options and
  Reset to the Remote Branch action follow IntelliJ Community (Apache-2.0),
  https://github.com/JetBrains/intellij-community at tag pycharm/2026.2.3
  (404a109fe630a2b9013f10b71081f87c9dbb32e2). Sources under
  plugins/git4idea/backend/src/update/: GitUpdateOptionsDialog.kt,
  GitUpdateOptionsPanel.kt, GitResetUpdater.kt and GitUpdater.java.
  The button and confirmation text come from
  plugins/git4idea/shared/resources/messages/GitBundle.properties.
  Reimplemented in Rust with GPUI and Git, including a separate confirmation
  and a checkout that protects local changes and untracked files.
