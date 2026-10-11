/**
 * The last-seen copy of a remote machine, written the way the TypeScript sidebar writes it.
 *
 * `sidebar_replay`'s last-seen pass runs this and compares the text with what the Rust writer
 * produces for the same rows. It exists because the two writers' agreement is a claim about BYTES
 * and the Rust-only check could not see it: `serde_json` writes an f64 through `ryu`, so a
 * `sidebarOrder` of zero reached the row as `0.0` where `JSON.stringify` writes `0`, every publish
 * rewrote a row it should have skipped, and a gate that compared `serde_json` with `serde_json`
 * called it identical. So the expected side is produced by `JSON.stringify` itself, which is what
 * `GpuiRemoteLastSeenStore.flush` calls in
 * `apps/desktop/sidebar/gxserver-runtime/helpers/remote-last-seen.ts`.
 *
 * Usage: `bun tooling/gx-core/last-seen-stringify.ts <frames.jsonl> <revision>`
 *
 * Recordings hold private data: nothing here prints the payload, only the finished text the caller
 * compares, and the caller keeps it out of the repository.
 */

const [framesPath, revisionText] = process.argv.slice(2);
if (!framesPath || !revisionText) {
  console.error('usage: last-seen-stringify.ts <frames.jsonl> <revision>');
  process.exit(2);
}
const revision = Number(revisionText);
if (!Number.isFinite(revision)) {
  console.error(`not a revision: ${revisionText}`);
  process.exit(2);
}

const text = await Bun.file(framesPath).text();
let snapshot: Record<string, unknown> | null = null;
for (const line of text.split('\n')) {
  if (!line.trim()) continue;
  const frame = JSON.parse(line) as { type?: string; snapshot?: Record<string, unknown> };
  if (frame.type === 'presentationSnapshot' && frame.snapshot) {
    snapshot = frame.snapshot;
    break;
  }
}
if (!snapshot) {
  console.error(`no presentationSnapshot frame in ${framesPath}`);
  process.exit(1);
}

snapshot.revision = revision;
const sessions = snapshot.sessions;
if (Array.isArray(sessions)) {
  // `orderPresentationSessions` (`packages/shared/gxserver-presentation-cache.ts`) re-sorts the
  // cache into this key sequence after every delta, which is the order the stored copy is in. The
  // one thing not copied from it is `localeCompare`: the Rust writer follows the DAEMON's byte
  // order on purpose (see `sort_projects`), and that difference is a settled decision this
  // artefact is not here to relitigate. Comparing code units rather than bytes only diverges above
  // the BMP, which no id reaches.
  const key = (session: Record<string, string>) =>
    [session.projectId, session.groupId, session.sortKey, session.sessionId].join('\u0000');
  sessions.sort((left, right) => {
    const l = key(left as Record<string, string>);
    const r = key(right as Record<string, string>);
    return l < r ? -1 : l > r ? 1 : 0;
  });
}

process.stdout.write(JSON.stringify(snapshot));
