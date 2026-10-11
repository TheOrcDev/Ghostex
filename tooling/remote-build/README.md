# Remote Linux builds on Blacksmith

`cargo xtask remote-start` builds the Linux x64 desktop app on a rented
[Blacksmith](https://www.blacksmith.sh) machine (16 cores, 64 GB, billed per
minute), downloads only the files that changed, and installs and launches the
result exactly like `cargo xtask start`. Use it when your own computer is too
slow to build Ghostex.

## One-time setup

Blacksmith only runs jobs for repositories owned by a GitHub organization, and
the snapshots it builds contain your uncommitted work, so builds go to a
private repository in an organization, never to the public `maddada/ghostex`.

1. Create a free organization: <https://github.com/organizations/new> (Free plan).
2. Create an empty private repository in it:
   `gh repo create <org>/ghostex-builds --private`
3. Sign in at <https://app.blacksmith.sh> with GitHub and install the Blacksmith
   GitHub app on that organization (access to `ghostex-builds` is enough).
4. Run the first build, which also saves the repository for later runs:
   `cargo xtask remote-start --repo <org>/ghostex-builds`

## Every day

```sh
cargo xtask remote-start               # build remotely, install and launch here
cargo xtask remote-start --build-only  # only update apps/desktop/build/linux/Ghostex
cargo xtask start --install-only       # install that staged build later
```

## What happens

1. A snapshot of the working tree, uncommitted and untracked files included, is
   committed in a private copy of the git index (your index, working tree and
   branch are not touched) and force-pushed to the `remote-build/linux-x64`
   branch of the build repository.
2. `remote-start` then starts `tooling/remote-build/linux-x64.yml` on that
   branch on Blacksmith (the build repository's `main` holds only that
   workflow, because GitHub needs it there to start it by hand). Two
   sticky disks (persistent disks) keep the checkout with its `target/`
   folders, and the Rust, Zig and Bun toolchains and caches, between runs, so
   only what changed is rebuilt.
3. The machine compares its build with the checksum list of your staged app and
   uploads only the files that differ (usually the two binaries, not the 1.4 GB
   of CEF). The download is checked file by file before anything is replaced,
   and the GitHub artifact is deleted afterwards.

## Expect

- The first build runs cold and can take an hour or more. Later builds reuse
  the sticky disks.
- If a run sits on "Waiting for a Blacksmith machine", the Blacksmith app is not
  installed on the organization.
- Sticky disks cost $0.50/GB per month and are deleted after 7 days without a
  build. To start from clean disks, change the `-v1` suffix of their keys in
  `linux-x64.yml`.
- A submodule (`.dependencies/zmx`, `zed`, `cef-rs`, `gpui-component`,
  `code-server`) checked out at a different commit is built at that commit, but
  it must be pushed to GitHub, and uncommitted edits inside a submodule are not
  included.
- Linux x64 only for now; macOS and Windows still build locally.
