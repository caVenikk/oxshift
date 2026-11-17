- crates/platform/vcs-ui/src/changes_browser.rs,
  crates/platform/vcs-ui/src/commit/dialogs.rs and
  crates/platform/workbench/src/workbench/commit.rs follow the rollback
  selection, toolbar and tree behavior of IntelliJ Community (Apache-2.0),
  https://github.com/JetBrains/intellij-community at tag pycharm/2026.2.3
  (404a109fe630a2b9013f10b71081f87c9dbb32e2).
  Sources under platform/vcs-impl/src/com/intellij/openapi/vcs/changes/:
  actions/RollbackAction.java, ui/RollbackChangesDialog.kt,
  ui/LocalChangesBrowser.java and ui/ChangesBrowserBase.java.
  The initially empty inclusion set follows
  platform/vcs-impl/shared/src/com/intellij/openapi/vcs/changes/ui/DefaultInclusionModel.kt.
  Reimplemented in Rust with GPUI. Push and Rollback share the directory
  tree and toolbar. Rollback uses the active context to include files and
  compares a local document with its committed snapshot.
