import { ClientStorageError, type StorageBackend, type StorageRecord, type StoreDefinition } from './types';

export const KiB = 1024;
export const MiB = 1024 * KiB;
export const STORAGE_BUDGETS: Record<StorageBackend, number> = {
  local: 2 * MiB,
  session: 2 * MiB,
  indexeddb: 128 * MiB,
};

/** Conservative UTF-16 key/value accounting; this is an app budget, not a claim about a browser's exact quota. */
export function storageBytes(key: string, raw: string): number {
  return 2 * (key.length + raw.length);
}

export function admission(
  definition: StoreDefinition,
  next: StorageRecord,
  rows: readonly StorageRecord[],
  backendBytes: number,
  now = Date.now()
): string[] {
  if (next.bytes > definition.maxEntryBytes) {
    throw new ClientStorageError('budget', definition.id, `${definition.owner} exceeds its entry size limit.`);
  }
  const previous = rows.find((row) => row.key === next.key);
  const others = rows.filter((row) => row.key !== next.key);
  let bytes = others.reduce((total, row) => total + row.bytes, next.bytes);
  let total = backendBytes - (previous?.bytes ?? 0) + next.bytes;
  let count = others.length + 1;
  const expired = (row: StorageRecord) => definition.maxAgeMs !== null && now - row.updatedAt > definition.maxAgeMs;
  const removals: string[] = [];
  if (definition.policy === 'cache') {
    for (const row of [...others, next].sort((a, b) => a.updatedAt - b.updatedAt || a.key.localeCompare(b.key))) {
      if (
        !expired(row) &&
        bytes <= definition.maxBytes &&
        count <= definition.maxEntries &&
        total <= STORAGE_BUDGETS[definition.backend]
      )
        break;
      removals.push(row.key);
      bytes -= row.bytes;
      total -= row.bytes;
      count--;
    }
  }
  if (bytes > definition.maxBytes || count > definition.maxEntries || total > STORAGE_BUDGETS[definition.backend]) {
    throw new ClientStorageError('budget', definition.id, `${definition.owner} has reached its storage budget.`);
  }
  return removals;
}
