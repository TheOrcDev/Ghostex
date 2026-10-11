# Submodule repair: code-server and zmx stranded at their old top-level path

Referenced from `AGENTS.md` ("Repository layout"). Moved here verbatim on 2026-09-18 to keep `AGENTS.md` inside the agent context budget.

The restructure also moved the `code-server` and `zmx` **submodule** gitlinks into `.dependencies/`. Git cannot move a submodule's working tree as part of a gitlink rename, so a checkout that had any of them initialized before 2026-08-22 keeps the real tree at the old top-level path (now untracked) and gets an empty directory at the new one. A fresh clone is unaffected. `prepare-macos-runtime.sh` now hard-fails on this signature instead of packaging an app with a dead Code tab.

Fast unblock, no move needed (`ZMX_ROOT=` for zmx):

```sh
GHOSTEX_CODE_SERVER_ROOT=$PWD/code-server cargo xtask start
```

Proper repair — move the tree and fix its git pointers. code-server needs **four** fixes because of the nested `lib/vscode` submodule; repairing them is not cosmetic, since a broken gitdir degrades `rev-parse HEAD` to `development` in the build fingerprint and forces a full VS Code rebuild:

```sh
rmdir .dependencies/code-server
mv code-server .dependencies/code-server
echo 'gitdir: ../../.git/modules/code-server' > .dependencies/code-server/.git
git config -f .git/modules/code-server/config \
  core.worktree ../../../.dependencies/code-server
echo 'gitdir: ../../../../.git/modules/code-server/modules/lib/vscode' \
  > .dependencies/code-server/lib/vscode/.git
git config -f .git/modules/code-server/modules/lib/vscode/config \
  core.worktree ../../../../../../.dependencies/code-server/lib/vscode
```

`zmx` has no nested submodule, so it needs only the first two:

```sh
rmdir .dependencies/zmx && mv zmx .dependencies/zmx
echo 'gitdir: ../../.git/modules/zmx' > .dependencies/zmx/.git
git config -f .git/modules/zmx/config core.worktree ../../../.dependencies/zmx
```

Verify: `git -C .dependencies/code-server rev-parse HEAD` prints `390f119a145e…`, and `git submodule status .dependencies/code-server` shows a leading space (not `-` or `+`).
