import type { StorageEvent } from './types';

const MAX_EVENTS = 200;
const events: StorageEvent[] = [];
const minuteWrites = new Map<string, { second: number; count: number }[]>();
const latestFailures = new Map<string, StorageEvent>();
const listeners = new Set<() => void>();
export const diagnosticPageId = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
let sequence = 0;
let scheduled = false;
let diagnosticSink: ((event: StorageEvent) => void) | undefined;
let activityChanged: (() => void) | undefined;

export type StorageActivity = {
  writes: [string, { second: number; count: number }[]][];
  failures: StorageEvent[];
  events: StorageEvent[];
};

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
export function mergeStorageEvents(incoming: StorageEvent[]): void {
  const known = new Set(events.map((event) => event.id));
  for (const entry of incoming) {
    if (known.has(entry.id)) continue;
    known.add(entry.id);
    events.push(entry);
    if (entry.operation === 'failure' && entry.at >= (latestFailures.get(entry.store)?.at ?? 0))
      latestFailures.set(entry.store, entry);
  }
  events.sort((a, b) => a.at - b.at);
  if (events.length > MAX_EVENTS) events.splice(0, events.length - MAX_EVENTS);
}
export function recordStorageEvent(event: Omit<StorageEvent, 'at' | 'id'>): void {
  const entry = { ...event, at: Date.now(), id: `${diagnosticPageId}:${++sequence}` };
  if (entry.operation === 'write') {
    const second = Math.floor(entry.at / 1_000);
    const buckets = (minuteWrites.get(entry.store) ?? []).filter((bucket) => bucket.second > second - 60);
    const last = buckets.at(-1);
    if (last?.second === second) last.count++;
    else buckets.push({ second, count: 1 });
    minuteWrites.set(entry.store, buckets);
  }
  mergeStorageEvents([entry]);
  try {
    diagnosticSink?.(entry);
  } catch (error) {
    console.error('[client-storage] Diagnostic sink failed.', error);
  }
  activityChanged?.();
  notifyStorage();
}
export function localStorageActivity(): StorageActivity {
  const cutoff = Math.floor(Date.now() / 1_000) - 60;
  return {
    writes: [...minuteWrites].map(([store, buckets]) => [store, buckets.filter((bucket) => bucket.second > cutoff)]),
    failures: [...latestFailures.values()].filter((event) => event.id.startsWith(`${diagnosticPageId}:`)),
    events: events.filter((event) => event.id.startsWith(`${diagnosticPageId}:`)),
  };
}
export function onLocalStorageActivity(callback: () => void): void {
  activityChanged = callback;
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
