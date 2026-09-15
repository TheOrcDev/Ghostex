import { installDatabaseGuard } from './database';
import { ClientStorageError } from '../types';
import { recordStorageEvent } from '../diagnostics';

export type BrowserBackend = 'local' | 'session';
const areas: Partial<Record<BrowserBackend, Storage>> = {};
let native:
  { get: Storage['getItem']; set: Storage['setItem']; remove: Storage['removeItem']; key: Storage['key'] } | undefined;
function area(backend: BrowserBackend): Storage {
  if (typeof window === 'undefined') throw new ClientStorageError('unavailable', '', 'Browser storage is unavailable.');
  if (!native)
    native = {
      get: Storage.prototype.getItem,
      set: Storage.prototype.setItem,
      remove: Storage.prototype.removeItem,
      key: Storage.prototype.key,
    };
  return (areas[backend] ??= backend === 'local' ? window.localStorage : window.sessionStorage);
}
export function readBrowser(backend: BrowserBackend, key: string): string | null {
  const storage = area(backend);
  return native!.get.call(storage, key);
}
export function writeBrowser(backend: BrowserBackend, key: string, raw: string | null): void {
  const storage = area(backend);
  if (raw === null) native!.remove.call(storage, key);
  else native!.set.call(storage, key, raw);
}
export function scanBrowser(backend: BrowserBackend): [string, string][] {
  const storage = area(backend);
  const entries: [string, string][] = [];
  for (let index = 0; index < storage.length; index++) {
    const key = native!.key.call(storage, index);
    if (key === null) continue;
    const raw = native!.get.call(storage, key);
    if (raw !== null) entries.push([key, raw]);
  }
  return entries;
}
export function subscribeBrowser(callback: (backend: BrowserBackend, key: string | null) => void): void {
  window.addEventListener('storage', (event) => {
    if (event.storageArea === area('local')) callback('local', event.key);
    else if (event.storageArea === area('session')) callback('session', event.key);
  });
}
let guarded = false;
/** A dependency cannot bypass budgets with Storage.prototype.setItem in development. */
export function installBrowserGuard(): void {
  if (guarded || typeof window === 'undefined') return;
  area('local');
  area('session');
  guarded = true;
  const reject = (operation: string, key?: string): never => {
    recordStorageEvent({ store: key ?? 'unknown', operation: 'unexpected', bytes: 0, reason: 'unregistered' });
    throw new ClientStorageError(
      'unregistered',
      key ?? '',
      `Direct browser storage ${operation} is forbidden. Register a store in packages/client-storage/catalog.ts.`
    );
  };
  Storage.prototype.setItem = function (key) {
    reject('write', String(key));
  };
  Storage.prototype.removeItem = function (key) {
    reject('remove', String(key));
  };
  Storage.prototype.clear = function () {
    reject('clear');
  };
  for (const backend of ['local', 'session'] as const) {
    const storage = area(backend);
    const proxy = new Proxy(storage, {
      get(target, name) {
        const value = Reflect.get(target, name, target);
        return typeof value === 'function' ? value.bind(target) : value;
      },
      set(_target, name) {
        return reject('property write', String(name));
      },
      deleteProperty(_target, name) {
        return reject('property remove', String(name));
      },
      defineProperty(_target, name) {
        return reject('property definition', String(name));
      },
    });
    Object.defineProperty(window, backend === 'local' ? 'localStorage' : 'sessionStorage', {
      configurable: true,
      get: () => proxy,
    });
  }
  installDatabaseGuard();
}
