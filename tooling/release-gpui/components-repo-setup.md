# Component releases in `maddada/ghostex-components`: one-time setup

Since 2026-09-16 the on-demand runtime components (the `cef-<version>` and
`code-server-<identity>` tags with their per-platform `.tar.gz` assets) are
published to the separate public repository
[maddada/ghostex-components](https://github.com/maddada/ghostex-components), so
they no longer appear in the releases list of `maddada/Ghostex`. App releases,
customer download links, the Sparkle feed, and the version-scoped
`gxserver-linux-*` assets stay on `maddada/Ghostex`.

The repository name is defined once, in
`tooling/release-gpui/components-repo.mjs` (`COMPONENTS_GITHUB_REPO`). Shell
and PowerShell scripts and the workflows resolve it by running that file, and
`GHOSTEX_COMPONENTS_REPO=<owner>/<repo>` overrides it everywhere for a test
repository. Every sealed `on-demand-resources.json` records the repository per
component (`components.<name>.githubRepo`), so apps shipped before the move keep
downloading from `maddada/Ghostex`, where their component releases stay until no
supported install references them.

## Secret: `COMPONENTS_GITHUB_TOKEN`

`github.token` cannot write releases in another repository, so every workflow
step that creates a component release or uploads a component asset uses this
secret. Read-only probes of the public component releases keep using
`github.token`. The `prepare` job (`tooling/release-gpui/preflight.mjs`) and the
component and packaging jobs fail with a message naming the secret when it is
missing, before any build work starts.

Fine-grained personal access token:

- Resource owner: `maddada`.
- Repository access: **Only select repositories**, pick
  `maddada/ghostex-components`.
- Repository permissions: **Contents: Read and write**, **Metadata: Read** (added
  automatically).
- No account permissions.
- Expiration: pick the longest the account allows and note the date; a release
  dispatched after expiry fails in `prepare` with the secret name.

```sh
gh secret set COMPONENTS_GITHUB_TOKEN --repo maddada/Ghostex
```

Paste the token when prompted, or pipe it in: `gh secret set
COMPONENTS_GITHUB_TOKEN --repo maddada/Ghostex < token.txt`.

Locally, `publish-component.mjs` and `mirror-component-release.mjs` use the
token when `COMPONENTS_GITHUB_TOKEN` is exported and the ambient `gh` login
otherwise.

## The repository itself

`maddada/ghostex-components` is public and must have at least one commit (a
README) before the first release can be created: GitHub refuses to create a
release tag in an empty repository. The README says what the repository is and
links to the Ghostex releases page; nothing else lives there.

Component releases are created with `--latest=false`, so the repository never
advertises a "latest" release.

## Mirroring a component that was published to `maddada/Ghostex`

New app builds reference the components repository only, so a component that
exists solely in `maddada/Ghostex` must be mirrored before a release reuses it.
The reuse probes (`plan.mjs`, the component workflows, `preflight.mjs`) look in
the components repository alone and would otherwise rebuild it.

```sh
bun run release:component:mirror -- --tag cef-148.0.10-g7ee53f5-chromium-148.0.7778.218 --dry-run
COMPONENTS_GITHUB_TOKEN=... bun run release:component:mirror -- --tag cef-148.0.10-g7ee53f5-chromium-148.0.7778.218
```

The script downloads every asset of the source release, checks each against
GitHub's digest metadata and its filename-bound `.sha256` sidecar, creates the
destination release with the same title and body (never as latest), uploads the
missing assets, and re-reads the destination to prove sizes and digests match.
It is idempotent: assets already present with the same bytes are no-ops, and an
asset present with different bytes aborts the run.

`bun run release:verify -- <version>` accepts component tags in either
repository, so releases published before the move keep verifying.
