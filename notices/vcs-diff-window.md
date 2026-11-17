- crates/platform/vcs-ui/src/diff/snapshot.rs: a resizable comparison window
  owned by the file-list dialog, with initial focus in the diff and focus
  returned to the owner when dismissed. Reimplemented for GPUI following
  IntelliJ Community (Apache-2.0), https://github.com/JetBrains/intellij-community
  at tag pycharm/2026.2.3 (404a109fe630a2b9013f10b71081f87c9dbb32e2).
  Sources: platform/diff-impl/src/com/intellij/diff/impl/DiffWindowBase.java,
  DiffWindow.java in the same package, and
  platform/vcs-impl/src/com/intellij/openapi/vcs/changes/ui/ChangesBrowserBase.java.
