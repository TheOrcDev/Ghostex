import { definitionForKey, definitions, storageCatalog } from '../catalog';
import { admission, storageBytes, STORAGE_BUDGETS } from '../budgets';
import { ClientStorageError, type StorageChange, type StorageRecord } from '../types';
import type { Mutation } from './database';
type Usage = { bytes: number; entries: number };
const empty = (): Usage => ({ bytes: 0, entries: 0 });
export interface StorageTable {
  get(key: string): Promise<StorageRecord | undefined>;
  byStore(store: string): Promise<StorageRecord[]>;
  put(row: StorageRecord): Promise<unknown>;
  delete(key: string): Promise<unknown>;
}
export interface StorageMetadata {
  get(key: string): Promise<any>;
  put(value: unknown, key: string): Promise<unknown>;
}

export async function applyDatabaseMutations(
  records: StorageTable,
  metadata: StorageMetadata,
  mutations: readonly Mutation[],
  migration: boolean
): Promise<StorageChange[]> {
  let revision = ((await metadata.get('revision')) as number | undefined) ?? 0;
  let total = ((await metadata.get('total')) as number | undefined) ?? 0;
  const changes: StorageChange[] = [];
  for (const mutation of mutations) {
    const definition = storageCatalog[mutation.store];
    if (definitionForKey(mutation.key)?.id !== definition.id || definition.backend !== 'indexeddb')
      throw new ClientStorageError('unregistered', definition.id, 'This store does not own the requested key.');
    const previous = (await records.get(mutation.key)) as StorageRecord | undefined;
    if (mutation.onlyIfAbsent && previous) continue;
    if (
      (previous?.raw === mutation.raw && previous?.schemaVersion === definition.version) ||
      (!previous && mutation.raw === null)
    )
      continue;
    const usage = ((await metadata.get(definition.id)) as Usage | undefined) ?? empty();
    const now = Date.now();
    let next =
      mutation.raw === null
        ? null
        : {
            key: mutation.key,
            store: definition.id,
            raw: mutation.raw,
            bytes: storageBytes(mutation.key, mutation.raw),
            updatedAt: now,
            revision: ++revision,
            schemaVersion: definition.version,
          };
    if (next && definition.retainedAt) {
      const timestamp = definition.retainedAt(next.raw);
      if (Number.isFinite(timestamp)) next.updatedAt = timestamp;
    }
    if (next && !migration) {
      if (total - (previous?.bytes ?? 0) + next.bytes > STORAGE_BUDGETS.indexeddb) {
        const disposable: StorageRecord[] = [];
        for (const cache of definitions.filter(
          (entry) => entry.backend === 'indexeddb' && entry.policy === 'cache' && entry.id !== definition.id
        ))
          disposable.push(...((await records.byStore(cache.id)) as StorageRecord[]));
        for (const entry of disposable.sort((a, b) => a.updatedAt - b.updatedAt)) {
          if (total - (previous?.bytes ?? 0) + next.bytes <= STORAGE_BUDGETS.indexeddb) break;
          const cacheUsage = (await metadata.get(entry.store)) as Usage;
          await records.delete(entry.key);
          cacheUsage.bytes -= entry.bytes;
          cacheUsage.entries--;
          total -= entry.bytes;
          await metadata.put(cacheUsage, entry.store);
          changes.push({ key: entry.key, store: entry.store, raw: null, revision: ++revision });
        }
      }
      let removals: string[] = [];
      if (definition.policy === 'cache') {
        const rows: StorageRecord[] = await records.byStore(definition.id);
        removals = admission(definition, next, rows, total, now);
        if (removals.includes(next.key)) {
          removals = removals.filter((key) => key !== next!.key);
          next = null;
        }
      } else {
        // Existing protected data may exceed a new budget. Shrinking it must still work.
        const grows = next.bytes > (previous?.bytes ?? 0);
        if (
          (next.bytes > definition.maxEntryBytes ||
            usage.bytes - (previous?.bytes ?? 0) + next.bytes > definition.maxBytes ||
            usage.entries + (previous ? 0 : 1) > definition.maxEntries ||
            total - (previous?.bytes ?? 0) + next.bytes > STORAGE_BUDGETS.indexeddb) &&
          grows
        )
          throw new ClientStorageError('budget', definition.id, `${definition.owner} has reached its storage budget.`);
      }
      for (const key of removals) {
        const row = (await records.get(key)) as StorageRecord;
        await records.delete(key);
        total -= row.bytes;
        usage.bytes -= row.bytes;
        usage.entries--;
        changes.push({ key, store: definition.id, raw: null, revision: ++revision });
      }
    }
    total -= previous?.bytes ?? 0;
    usage.bytes -= previous?.bytes ?? 0;
    if (previous) usage.entries--;
    if (next) {
      next.revision = ++revision;
      await records.put(next);
      total += next.bytes;
      usage.bytes += next.bytes;
      usage.entries++;
    } else await records.delete(mutation.key);
    await metadata.put(usage, definition.id);
    changes.push({ key: mutation.key, store: definition.id, raw: next?.raw ?? null, revision: ++revision });
  }
  await metadata.put(total, 'total');
  await metadata.put(revision, 'revision');
  return changes;
}

