- crates/platform/vcs-ui/src/push.rs, crates/platform/git/src/remote.rs,
  crates/platform/workbench/src/workbench/remote.rs and the Push keymaps:
  the outgoing commit selection, changes browser, details pane, comparison
  and dialog layout follow IntelliJ Community (Apache-2.0),
  https://github.com/JetBrains/intellij-community at tag pycharm/2026.2.3
  (404a109fe630a2b9013f10b71081f87c9dbb32e2).
  Sources: platform/dvcs-impl/src/com/intellij/dvcs/push/ui/PushLog.java,
  PushLogChangesBrowser.kt and VcsPushDialog.java;
  platform/dvcs-api/src/com/intellij/dvcs/push/PushSettings.java;
  platform/vcs-impl/src/com/intellij/openapi/vcs/changes/ui/EditSourceForDialogAction.java
  and changes/committed/CommittedChangesTreeBrowser.java in the same VCS module.
  Reimplemented in Rust with GPUI and immutable Git blobs.
