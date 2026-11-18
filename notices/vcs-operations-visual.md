- crates/platform/vcs-ui/src/operations.rs, crates/platform/workbench/src/
  workbench/remote.rs, crates/platform/workbench/keymaps/{macos,default}/
  vcs-push.toml, crates/platform/ui/src/menu.rs and crates/platform/themes/src/
  {roles.rs,import/mapping.rs}
  The VCS Operations labels, row order, icon associations, numbered row layout,
  popup header colour and compare shortcut follow IntelliJ Platform Community
  (Apache-2.0), https://github.com/JetBrains/intellij-community @ tag
  pycharm/2026.2.3 (404a109fe630a2b9013f10b71081f87c9dbb32e2):
  plugins/git4idea/backend/src/actions/GitQuickListContentProvider.java,
  platform/vcs-impl/resources/META-INF/VcsActions.xml,
  platform/dvcs-impl/src/com/intellij/dvcs/actions/CopyCurrentBranchNameAction.kt,
  platform/platform-resources-en/src/messages/ActionsBundle.properties,
  platform/util/ui/src/com/intellij/util/ui/JBUI.java and
  platform/platform-resources/src/keymaps/.
  Behaviour, presentation and strings reimplemented in Rust with GPUI;
  existing licensed icons are reused. Compare is scoped to the popup and
  preserves the selected row's file argument; unavailable actions stay disabled.
