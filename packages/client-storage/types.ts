export type StorageBackend = 'local' | 'session' | 'indexeddb';
export type StoragePolicy = 'protected' | 'preference' | 'cache';
export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

export interface StorageCodec<T> {
  readonly schema: string;
  decode(raw: string): T;
  encode(value: T): string;
}

export interface StoreDefinition<T = unknown> {
  readonly id: string;
  readonly owner: string;
  readonly source: string;
  readonly backend: StorageBackend;
  readonly policy: StoragePolicy;
  readonly version: number;
  readonly key: string;
  readonly collection: boolean;
  /** A dependency writes these keys itself through the browser Storage API; the development guard forwards and meters those writes instead of rejecting them. */
  readonly external: boolean;
  readonly maxEntryBytes: number;
  readonly maxBytes: number;
  readonly maxEntries: number;
  readonly maxAgeMs: number | null;
  readonly codec: StorageCodec<T>;
  readonly retainedAt?: (raw: string) => number;
  readonly upgrade?: (raw: string, fromVersion: number) => string;
}

export type StorageRecord = {
  key: string;
  store: string;
  raw: string;
  bytes: number;
  updatedAt: number;
  revision: number;
  schemaVersion: number;
};
export type StorageChange = { store: string; key: string; raw: string | null; revision: number };
export type StorageFailure = 'quota' | 'budget' | 'schema' | 'unavailable' | 'unregistered' | 'transaction';
export class ClientStorageError extends Error {
  constructor(
    readonly reason: StorageFailure,
    readonly store: string,
    message: string
  ) {
    super(message);
    this.name = 'ClientStorageError';
  }
}
export type StorageEvent = {
  id: string;
  at: number;
  store: string;
  operation: 'write' | 'remove' | 'migrate' | 'expire' | 'failure' | 'unexpected';
  bytes: number;
  reason?: StorageFailure;
};
export type StoreUsage = {
  id: string;
  owner: string;
  source: string;
  backend: StorageBackend;
  policy: StoragePolicy;
  schema: string;
  maxBytes: number;
  maxEntries: number;
  bytes: number;
  entries: number;
  pending: number;
  pendingBytes: number;
  writesPerMinute: number;
  lastFailure?: StorageEvent;
  largest: { key: string; bytes: number; updatedAt?: number }[];
};
export type StorageInspection = {
  measuredAt: number;
  ready: boolean;
  stores: StoreUsage[];
  unknown: { backend: StorageBackend; key: string; bytes: number }[];
  events: StorageEvent[];
  totals: Record<StorageBackend, { bytes: number; budget: number }>;
};
