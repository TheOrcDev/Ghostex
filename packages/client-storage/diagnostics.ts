import type { StorageEvent } from './types';

const MAX_EVENTS = 200;
const events: StorageEvent[] = [];
const minuteWrites = new Map<string, { second: number; count: number }[]>();
const latestFailures = new Map<string, StorageEvent>();
const listeners = new Set<() => void>();
let scheduled = false;
let diagnosticSink: ((event: StorageEvent) => void) | undefined;

export function notifyStorage(): void {
  if (scheduled) return;
  scheduled = true;
  queueMicrotask(() => {
    scheduled = false;
    for (const listener of listeners) {
      try {
        listener();
      } catch (error) {
        console.error('[client-storage] Diagnostic subscriber failed.', error);
      }
    }
  });
}
export function recordStorageEvent(event: Omit<StorageEvent, 'at'>): void {
  const entry = { ...event, at: Date.now() };
  if (entry.operation === 'failure') latestFailures.set(entry.store, entry);
  if (entry.operation === 'write') {
    const second = Math.floor(entry.at / 1_000);
    const buckets = (minuteWrites.get(entry.store) ?? []).filter((bucket) => bucket.second > second - 60);
    const last = buckets.at(-1);
    if (last?.second === second) last.count++;
    else buckets.push({ second, count: 1 });
    minuteWrites.set(entry.store, buckets);
  }
  events.push(entry);
  if (events.length > MAX_EVENTS) events.splice(0, events.length - MAX_EVENTS);
  try {
    diagnosticSink?.(entry);
  } catch (error) {
    console.error('[client-storage] Diagnostic sink failed.', error);
  }
  notifyStorage();
}
export function storageEvents(): StorageEvent[] {
  return events.slice();
}
export function subscribeStorage(callback: () => void): () => void {
  listeners.add(callback);
  return () => {
    listeners.delete(callback);
  };
}
/** The host sink must apply its debug/scenario gate before writing routine events to disk. */
export function setStorageDiagnosticSink(sink?: (event: StorageEvent) => void): void {
  diagnosticSink = sink;
}

export function writesInLastMinute(store: string): number {
  const cutoff = Math.floor(Date.now() / 1_000) - 60;
  return (minuteWrites.get(store) ?? []).reduce((sum, bucket) => sum + (bucket.second > cutoff ? bucket.count : 0), 0);
}
export function lastStorageFailure(store: string): StorageEvent | undefined {
  return latestFailures.get(store);
}
