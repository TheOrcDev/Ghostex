import { owns, storageCatalog, type StoreId } from './catalog';
import { atomicStorage, flushClientStorage, managedKeys, readManaged, subscribeManaged, writeManaged } from './service';
import { ClientStorageError, type StorageCodec } from './types';

type StoreValue<I extends StoreId> = (typeof storageCatalog)[I]['codec'] extends StorageCodec<infer T> ? T : never;

export function managedStore<I extends StoreId>(id: I) {
  const definition = storageCatalog[id];
  function key(suffix = ''): string {
    if (!definition.collection && suffix) throw new Error('A singleton store does not accept a key suffix.');
    return definition.key + suffix;
  }
  return {
    definition,
    get(suffix?: string): StoreValue<I> | undefined {
      const raw = readManaged(definition, key(suffix));
      return raw === null ? undefined : (definition.codec.decode(raw) as StoreValue<I>);
    },
    set(value: StoreValue<I>, suffix?: string): void {
      writeManaged(definition, key(suffix), (definition.codec as StorageCodec<StoreValue<I>>).encode(value));
    },
    async setDurable(value: StoreValue<I>, suffix?: string): Promise<void> {
      this.set(value, suffix);
      await flushClientStorage([id]);
    },
    remove(suffix?: string): void {
      writeManaged(definition, key(suffix), null);
    },
    entries(): [string, StoreValue<I>][] {
      return managedKeys([id]).flatMap((name) => {
        const raw = readManaged(definition, name);
        return raw === null ? [] : [[name.slice(definition.key.length), definition.codec.decode(raw) as StoreValue<I>]];
      });
    },
    async update(
      suffix: string,
      transform: (value: StoreValue<I> | undefined) => StoreValue<I> | undefined
    ): Promise<void> {
      await atomicStorage([id], () => {
        const next = transform(this.get(suffix));
        if (next === undefined) this.remove(suffix);
        else this.set(next, suffix);
      });
    },
    subscribe(callback: () => void): () => void {
      return subscribeManaged([id], callback);
    },
    flush: () => flushClientStorage([id]),
  };
}

/** Existing feature codecs may keep their serialized format, but can address only explicitly registered stores. */
export function storageScope<const I extends readonly StoreId[]>(ids: I) {
  const owner = (key: string) => {
    const definition = ids.map((id) => storageCatalog[id]).find((definition) => owns(definition, key));
    if (!definition) throw new ClientStorageError('unregistered', ids.join(', '), `Unregistered storage key: ${key}`);
    return definition;
  };
  return {
    getItem(key: string): string | null {
      return readManaged(owner(key), key);
    },
    setItem(key: string, raw: string): void {
      writeManaged(owner(key), key, raw);
    },
    removeItem(key: string): void {
      writeManaged(owner(key), key, null);
    },
    keys(): string[] {
      return managedKeys(ids);
    },
    key(index: number): string | null {
      return managedKeys(ids)[index] ?? null;
    },
    get length(): number {
      return managedKeys(ids).length;
    },
    subscribe(callback: (event: { key: string; newValue: string | null }) => void): () => void {
      return subscribeManaged(ids, (event) => callback({ key: event.key, newValue: event.raw }));
    },
    flush: () => flushClientStorage(ids),
  };
}
export type ScopedStorage = ReturnType<typeof storageScope>;
