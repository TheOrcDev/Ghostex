import { recordStorageEvent } from '../diagnostics';
import { definitionForKey, definitions, storageCatalog, type StoreId } from '../catalog';
import { admission, storageBytes, STORAGE_BUDGETS } from '../budgets';
import { ClientStorageError, type StorageChange, type StorageRecord } from '../types';

export type Mutation = { key: string; store: StoreId; raw: string | null; onlyIfAbsent?: boolean };
type Usage = { bytes: number; entries: number };
let opening: Promise<IDBDatabase> | undefined;
let nativeOpen: IDBFactory['open'] | undefined;
function openNative(name: string, version?: number): IDBOpenDBRequest {
  nativeOpen ??= indexedDB.open.bind(indexedDB);
  return nativeOpen(name, version);
}
export function installDatabaseGuard(): void {
  nativeOpen ??= indexedDB.open.bind(indexedDB);
  const reject = (name: string): never => {
    recordStorageEvent({ store: name, operation: 'unexpected', bytes: 0, reason: 'unregistered' });
    throw new ClientStorageError(
      'unregistered',
      name,
      'Direct database access is forbidden. Use a registered client storage handle.'
    );
  };
  IDBFactory.prototype.open = function (name) {
    return reject(name);
  };
  IDBFactory.prototype.deleteDatabase = function (name) {
    return reject(name);
  };
}
const DATABASE = 'ghostex-client-storage';
const empty = (): Usage => ({ bytes: 0, entries: 0 });
function request<T>(value: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    value.onsuccess = () => resolve(value.result);
    value.onerror = () => reject(value.error);
  });
}
export function openDatabase(): Promise<IDBDatabase> {
  if (!opening) {
    opening = new Promise<IDBDatabase>((resolve, reject) => {
      const value = openNative(DATABASE, 1);
      value.onupgradeneeded = () => {
        value.result.createObjectStore('records', { keyPath: 'key' }).createIndex('store', 'store');
        value.result.createObjectStore('metadata');
      };
      value.onsuccess = () => {
        const db = value.result;
        db.onversionchange = () => {
          db.close();
          opening = undefined;
        };
        resolve(db);
      };
      value.onerror = () => reject(value.error);
      value.onblocked = () =>
        reject(new ClientStorageError('unavailable', '', 'Close older Ghostex pages to finish upgrading storage.'));
    }).catch((error: unknown) => {
      opening = undefined;
      throw error;
    });
  }
  return opening;
}
export async function readDatabaseSnapshot(): Promise<{ rows: StorageRecord[]; revision: number }> {
  const db = await openDatabase();
  const transaction = db.transaction(['records', 'metadata']);
  const [rows, revision] = await Promise.all([
    request<StorageRecord[]>(transaction.objectStore('records').getAll()),
    request(transaction.objectStore('metadata').get('revision')),
  ]);
  return { rows, revision: revision ?? 0 };
}
export async function readDatabase(): Promise<StorageRecord[]> {
  return (await readDatabaseSnapshot()).rows;
}

async function apply(
  transaction: IDBTransaction,
  mutations: readonly Mutation[],
  migration: boolean
): Promise<StorageChange[]> {
  const records = transaction.objectStore('records');
  const metadata = transaction.objectStore('metadata');
  let revision = ((await request(metadata.get('revision'))) as number | undefined) ?? 0;
  let total = ((await request(metadata.get('total'))) as number | undefined) ?? 0;
  const changes: StorageChange[] = [];
  for (const mutation of mutations) {
    const definition = storageCatalog[mutation.store];
    if (definitionForKey(mutation.key)?.id !== definition.id || definition.backend !== 'indexeddb')
      throw new ClientStorageError('unregistered', definition.id, 'This store does not own the requested key.');
    const previous = (await request(records.get(mutation.key))) as StorageRecord | undefined;
    if (mutation.onlyIfAbsent && previous) continue;
    if (
      (previous?.raw === mutation.raw && previous?.schemaVersion === definition.version) ||
      (!previous && mutation.raw === null)
    )
      continue;
    const usage = ((await request(metadata.get(definition.id))) as Usage | undefined) ?? empty();
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
          disposable.push(...((await request(records.index('store').getAll(cache.id))) as StorageRecord[]));
        for (const entry of disposable.sort((a, b) => a.updatedAt - b.updatedAt)) {
          if (total - (previous?.bytes ?? 0) + next.bytes <= STORAGE_BUDGETS.indexeddb) break;
          const cacheUsage = (await request(metadata.get(entry.store))) as Usage;
          records.delete(entry.key);
          cacheUsage.bytes -= entry.bytes;
          cacheUsage.entries--;
          total -= entry.bytes;
          metadata.put(cacheUsage, entry.store);
          changes.push({ key: entry.key, store: entry.store, raw: null, revision: ++revision });
        }
      }
      let removals: string[] = [];
      if (definition.policy === 'cache') {
        const rows: StorageRecord[] = await request(records.index('store').getAll(definition.id));
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
        const row = (await request(records.get(key))) as StorageRecord;
        records.delete(key);
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
      records.put(next);
      total += next.bytes;
      usage.bytes += next.bytes;
      usage.entries++;
    } else records.delete(mutation.key);
    metadata.put(usage, definition.id);
    changes.push({ key: mutation.key, store: definition.id, raw: next?.raw ?? null, revision: ++revision });
  }
  metadata.put(total, 'total');
  metadata.put(revision, 'revision');
  return changes;
}

/** All pages share this strict transaction and its usage counters. No key enumeration on ordinary protected writes. */
export async function commitDatabase(mutations: readonly Mutation[], migration = false): Promise<StorageChange[]> {
  return transactionDatabase(() => mutations, [], migration);
}
export async function transactionDatabase(
  action: (rows: StorageRecord[]) => readonly Mutation[],
  ids: readonly StoreId[],
  migration = false
): Promise<StorageChange[]> {
  const db = await openDatabase();
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(['records', 'metadata'], 'readwrite', { durability: 'strict' });
    let result: StorageChange[] = [];
    let failure: unknown;
    transaction.oncomplete = () => resolve(result);
    transaction.onabort = () =>
      reject(failure ?? transaction.error ?? new Error('Storage transaction was interrupted.'));
    transaction.onerror = () => {
      failure ??= transaction.error;
    };
    void (async () => {
      const rows: StorageRecord[] = [];
      for (const id of ids)
        rows.push(
          ...((await request(transaction.objectStore('records').index('store').getAll(id))) as StorageRecord[])
        );
      result = await apply(transaction, action(rows), migration);
    })().catch((error: unknown) => {
      failure = error;
      transaction.abort();
    });
  });
}

/** Open a legacy database without creating an empty database on fresh installs. */
export async function readLegacyDatabase(
  name: string,
  store: string
): Promise<{ rows: unknown[]; close: () => void; retire: () => Promise<void> } | undefined> {
  const db = await new Promise<IDBDatabase | undefined>((resolve, reject) => {
    const value = openNative(name);
    let absent = false;
    value.onupgradeneeded = () => {
      absent = true;
      value.transaction!.abort();
    };
    value.onsuccess = () => resolve(value.result);
    value.onerror = () => (absent ? resolve(undefined) : reject(value.error));
    value.onblocked = () => reject(new Error(`Legacy storage ${name} is blocked.`));
  });
  if (!db) return undefined;
  if (!db.objectStoreNames.contains(store)) {
    db.close();
    return undefined;
  }
  const transaction = db.transaction(store);
  const table = transaction.objectStore(store);
  const [rows, keys] = await Promise.all([request(table.getAll()), request(table.getAllKeys())]);
  return {
    rows,
    close: () => db.close(),
    retire: () =>
      new Promise<void>((resolve, reject) => {
        const removal = db.transaction(store, 'readwrite', { durability: 'strict' });
        const records = removal.objectStore(store);
        for (let index = 0; index < rows.length; index++) {
          const current = records.get(keys[index]!);
          current.onsuccess = () => {
            if (JSON.stringify(current.result) === JSON.stringify(rows[index])) records.delete(keys[index]!);
          };
        }
        removal.oncomplete = () => {
          db.close();
          resolve();
        };
        removal.onabort = () => {
          db.close();
          reject(removal.error);
        };
        removal.onerror = () => {
          db.close();
          reject(removal.error);
        };
      }),
  };
}
