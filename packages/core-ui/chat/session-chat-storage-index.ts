import { storageScope, type ScopedStorage, type StoreId } from '@/packages/client-storage';
/**
 * CDXC:Drafts 2026-09-11 WHY:
 * Enumerating all localStorage keys on every save, acknowledgement, and recovery receipt multiplied a small text history into millions of synchronous reads.
 * Each page scans a namespace once, then indexes writes and cross-page storage events by session; durable storage remains authoritative.
 */
export class SessionChatStorageIndex<T> {
  private rows: Map<string, T> | undefined;
  private groups = new Map<string, Map<string, T>>();
  private listening = false;

  private readonly storage: ScopedStorage;

  constructor(
    store: StoreId,
    private readonly prefix: string,
    private readonly decode: (raw: string) => T | null,
    private readonly groupKey: (entry: T) => string
  ) { this.storage = storageScope([store]); }

  private update(key: string, raw: string | null): void {
    if (!this.rows) return;
    const previous = this.rows.get(key);
    if (previous) {
      const group = this.groups.get(this.groupKey(previous));
      group?.delete(key);
      if (group?.size === 0) this.groups.delete(this.groupKey(previous));
    }
    this.rows.delete(key);
    const entry = raw === null ? null : this.decode(raw);
    if (!entry) return;
    this.rows.set(key, entry);
    const name = this.groupKey(entry);
    let group = this.groups.get(name);
    if (!group) this.groups.set(name, (group = new Map()));
    group.set(key, entry);
  }

  private reset(): void {
    this.rows = undefined;
    this.groups.clear();
  }

  private ensureLoaded(): void {
    if (this.rows) return;
    if (!this.listening) {
      this.storage.subscribe((event) => {
        this.update(event.key, this.storage.getItem(event.key));
      });
      window.addEventListener('pageshow', (event) => {
        if (event.persisted) this.reset();
      });
      this.listening = true;
    }
    this.rows = new Map();
    try {
      const storage = this.storage;
      for (const key of storage.keys()) {
        if (key.startsWith(this.prefix)) this.update(key, storage.getItem(key));
      }
    } catch (error) {
      this.reset();
      throw error;
    }
  }

  entries(group?: string): [string, T][] {
    this.ensureLoaded();
    return [...(group === undefined ? this.rows!.entries() : (this.groups.get(group)?.entries() ?? []))];
  }

  set(key: string, value: T): void {
    const raw = JSON.stringify(value);
    this.storage.setItem(key, raw);
    this.update(key, raw);
  }

  refresh(key: string): void {
    this.update(key, this.storage.getItem(key));
  }

  remove(key: string): void {
    this.storage.removeItem(key);
    this.update(key, null);
  }
}
