# Managed client storage

All first-party browser persistence goes through this package. `catalog.ts` owns every namespace, its feature owner and source, schema version, backend, byte/count limits, retention, and disposal policy. Direct browser storage access is confined to the two adapters and rejected by `bun run storage:check` in typechecks, builds, and CI.

## Choosing a store

- Small preferences use managed localStorage, with a 2 MiB origin budget.
- Drafts, pending operations, recovery records, history, and growing caches use IndexedDB, with a 128 MiB application budget. Protected stores never expire or evict user work.
- Temporary directory caches use managed sessionStorage, with a 2 MiB budget.

Byte accounting conservatively counts UTF-16 key and serialized-value lengths. It is an application budget, not the browser's physical quota. IndexedDB admission and its usage counters are committed in the same origin-wide transaction. Disposable caches are the only records eligible for automatic eviction. Unknown data is reported and preserved.

## Adding a feature

Add a named definition to `catalog.ts` with the real owning source path and a codec that validates its value. Use the existing codec helpers or define a codec for the feature's domain type. Review limits against realistic data. User-authored data awaiting delivery is `protected`; it has no automatic retention deadline.

```ts
import { managedStore } from '@/packages/client-storage';

const wrapping = managedStore('codeWrap');
wrapping.set('1');
const enabled = wrapping.get() === '1';
```

Collection handles take a suffix within their own namespace. They cannot address another store. Existing features with established serialized codecs use `storageScope(['registeredId'])`; the scope validates the catalog schema and rejects keys outside those explicitly registered stores. Do not add a raw browser adapter or a generic unregistered key API in feature code.

For schema changes, increment the definition's version and provide its `upgrade(raw, fromVersion)` function. Upgrades validate and commit before the view opens. A newer unsupported schema or a missing upgrade is an explicit loading failure.

## Startup and durability

Call `bootClientStorage(start)` from the page entry point before mounting a view that reads IndexedDB-backed state. It hydrates the in-memory index and completes migration before calling `start`; a loading failure offers Retry without mounting an empty editor over existing work.

Reads are synchronous after hydration. IndexedDB writes update the page's working view immediately and queue a strict durable transaction. `set` means queued, not saved. Use `setDurable`, a handle's `flush`, or `flushClientStorage([relevantStoreIds])` before transferring ownership or releasing a page. An unrelated cache failure does not reject a scoped durability barrier.

Failed writes remain pending, retry with bounded backoff, and are visible through `storageFailure`, subscriptions, and the inspector. Draft, question-answer, and Docs editors show a save failure while retaining their text. Do not clear an editor merely because its working copy reads back correctly.

Use `handle.update(suffix, transform)` for a read/modify/write operation shared by several pages. Its synchronous transform reads the latest committed record under an IndexedDB write transaction. `atomicStorage` supports coordinated changes across registered stores, such as committing dismissal ranges and removing their individual recovery markers together. Do not await network work inside an atomic callback.

BroadcastChannel carries committed changes between pages. Revision tracking rejects stale messages; a focus or restored-page rescan reconciles missed events. Ordinary draft writes use indexed records and usage counters, not an enumeration of browser keys on every keystroke.

## Migration and diagnostics

Migration copies existing raw values into IndexedDB with a strict commit, reads them back, and removes a legacy source only when the copied value and current source match. Existing protected data may exceed a new budget; migration still preserves it, and subsequent shrinking writes remain possible. Legacy outbox and conversation databases are migrated through the same adapter. Unknown or conflicting sources remain available for inspection.

Settings > Advanced > Debugging > Show debug UI controls reveals Storage usage. It reports committed bytes separately from pending writes, feature ownership, budgets, entry counts, the largest keys, write rate, failures, and unknown entries. Its clear action accepts only disposable caches.

Diagnostic events contain metadata, not stored values. The event history is bounded to 200 in-memory events; write-rate counters retain 60 one-second buckets per registered store. A disk sink must apply the host's existing debug switch and expiring scenario gate. There is no automatic routine disk logging.

The development guard rejects direct Storage methods, named-property writes, and IndexedDB factory calls, including calls from dependencies. Fix the caller or register a deliberately owned store; do not add bypasses to the adapter allowlist.
