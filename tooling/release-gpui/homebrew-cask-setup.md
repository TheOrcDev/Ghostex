# Homebrew cask publishing: one-time setup

**Since 2026-10-04 the official-cask pull request is opt-in** (`--official`; user
decision): Homebrew's autobump bot updates the official cask on every release, and
this token's write access to the fork failed with HTTP 404 on every release from
10.0.1 on. The release workflow only pushes the personal tap now, and its audit
step (`brew style`/`brew audit`, which only fills in the PR template) is switched
off with `false &&` in `release-gpui-publish.yml`. Everything below describes the
`--official` path, kept for when it is wanted again (it needs the fork token
below); the tap push needs only `HOMEBREW_TAP_TOKEN` or `HOMEBREW_GITHUB_API_TOKEN`.

The macOS publish stage of `.github/workflows/release-gpui-publish.yml` runs
`tooling/release-gpui/publish-homebrew-cask.mjs`, which can update two casks:

- the official `ghostex` cask, `Casks/g/ghostex.rb` in
  [Homebrew/homebrew-cask](https://github.com/Homebrew/homebrew-cask), by opening a
  version-bump pull request from the `maddada/homebrew-cask` fork on a
  `ghostex-<version>` branch, titled `ghostex <version>`;
- the legacy personal tap, `Casks/ghostex.rb` in
  [maddada/homebrew-tap](https://github.com/maddada/homebrew-tap), by committing
  straight to `main` so `maddada/tap/ghostex` installs keep updating.

Both edits change `version` and `sha256` only. The sha256 is the digest GitHub
records for `ghostex-<version>-arm64.dmg`, cross-checked against
`release-provenance-<version>.json` when the release carries one.

## Secret: `HOMEBREW_GITHUB_API_TOKEN`

One GitHub token for the `maddada` account, stored as a repository secret on
`maddada/Ghostex`. The workflow step is skipped with a message when the secret is
missing, so nothing breaks before it is added.

Fine-grained personal access token (preferred):

- Resource owner: `maddada`.
- Repository access: **Only select repositories**, pick `maddada/homebrew-cask`
  and `maddada/homebrew-tap`.
- Repository permissions: **Contents: Read and write**, **Pull requests: Read and
  write**, **Metadata: Read** (added automatically).
- No account permissions, and no **Workflows** permission (the `workflow` scope
  of a classic token). The script never copies upstream commits into the fork,
  so it never has to write under `.github/workflows`; see the fork section.

A fine-grained token opens the pull request against `Homebrew/homebrew-cask`
even though that repository is not in its list: creating a PR only needs write
access to the fork branch and read access to the public upstream.

Classic token alternative: scope `public_repo` only. It grants push and PR rights
on every public repository the account can write to, so prefer the fine-grained
token.

The token must belong to the account that owns the fork: the script derives the
fork owner from `GET /user` with this token and pushes the bump branch there.

```sh
gh secret set HOMEBREW_GITHUB_API_TOKEN --repo maddada/Ghostex
```

Paste the token when prompted, or pipe it in: `gh secret set
HOMEBREW_GITHUB_API_TOKEN --repo maddada/Ghostex < token.txt`.

`HOMEBREW_TAP_TOKEN` is optional. When set, the tap push uses it instead of
`HOMEBREW_GITHUB_API_TOKEN`; use it only if the tap should be written by a
different account or a token that is not allowed to touch the fork.

## Fork of Homebrew/homebrew-cask

`maddada/homebrew-cask` already exists (it carried the `add-ghostex` submission).
The script forks the repository itself if it is missing. It never syncs the
fork: the `ghostex-<version>` branch is created (or force-reset, on a retry) at
the head commit of upstream's default branch, which the fork can reference
because a fork shares upstream's object network, and the bump is committed on
top of that commit. The fork's own `main` may fall arbitrarily far behind and
nothing reads it. Feature branches such as `add-ghostex` do not interfere.

The fork used to be fast-forwarded with the merge-upstream API before
branching. Release 9.7.0 failed there with HTTP 422, "refusing to allow a
Personal Access Token to create or update workflow
`.github/workflows/check-issues.yml` without `workflow` scope": that sync
copies every upstream commit into the fork, including Homebrew's frequent
workflow edits, which a token limited to contents and pull requests may not
write. Do not bring the sync back or widen the token for it; the pull request
never depends on the fork's `main` being current.

## Autobump

The cask has no `no_autobump!` and its `livecheck` block has no `skip`, so it is
in Homebrew's autobump set: BrewTestBot checks `appcast.xml` on `main` every
three hours and opens `ghostex <version>` itself when it sees a newer version.

Our PR is still the primary path: it lands minutes after the release, before the
next autobump sweep, with the sha256 verified against the release provenance.
The two never collide: `brew bump` skips a cask that already has an open bump
PR, and the script exits 0 with the existing PR URL when one is open for the
version, whoever opened it. If BrewTestBot beats the workflow (for example after
a publish-only recovery run hours later), the workflow logs that URL and does
nothing else. The script also exits 0 when the live cask already carries the
version.

## Running it by hand

```sh
node tooling/release-gpui/publish-homebrew-cask.mjs --version 9.5.1 --dry-run
HOMEBREW_GITHUB_API_TOKEN=... node tooling/release-gpui/publish-homebrew-cask.mjs --version 9.5.1 --publish --official
```

`--dry-run` prints the exact diff for both casks and performs no writes.
`--sha256 <hex>` overrides the digest lookup. The local brew-driven tap updater,
`tooling/release-gpui-homebrew.mjs`, still works for the personal tap alone.
