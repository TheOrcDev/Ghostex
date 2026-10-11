import { installDatabaseGuard } from './database';
import { ClientStorageError } from '../types';
import { recordStorageEvent } from '../diagnostics';

export type BrowserBackend = 'local' | 'session';
type BrowserAdapterState = {
  areas: Partial<Record<BrowserBackend, Storage>>;
  native?: { get: Storage['getItem']; set: Storage['setItem']; remove: Storage['removeItem']; key: Storage['key'] };
  guarded: boolean;
};
// Retain the original methods when a development module is hot-reloaded after the guard is installed.
const stateKey = Symbol.for('ghostex.client-storage.browser-adapter');
const globals = globalThis as typeof globalThis & { [stateKey]?: BrowserAdapterState };
const state = (globals[stateKey] ??= { areas: {}, guarded: false });
function area(backend: BrowserBackend): Storage {
  if (typeof window === 'undefined') throw new ClientStorageError('unavailable', '', 'Browser storage is unavailable.');
  if (!state.native)
    state.native = {
      get: Storage.prototype.getItem,
      set: Storage.prototype.setItem,
      remove: Storage.prototype.removeItem,
      key: Storage.prototype.key,
    };
  return (state.areas[backend] ??= backend === 'local' ? window.localStorage : window.sessionStorage);
}
export function readBrowser(backend: BrowserBackend, key: string): string | null {
  const storage = area(backend);
  return state.native!.get.call(storage, key);
}
export function writeBrowser(backend: BrowserBackend, key: string, raw: string | null): void {
  const storage = area(backend);
  if (raw === null) state.native!.remove.call(storage, key);
  else state.native!.set.call(storage, key, raw);
}
export function scanBrowser(backend: BrowserBackend): [string, string][] {
  const storage = area(backend);
  const entries: [string, string][] = [];
  for (let index = 0; index < storage.length; index++) {
    const key = state.native!.key.call(storage, index);
    if (key === null) continue;
    const raw = state.native!.get.call(storage, key);
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
export type BrowserWriteForwarder = (backend: BrowserBackend, key: string, raw: string | null) => boolean;
/**
 * A dependency cannot bypass budgets with Storage.prototype.setItem in development.
 * CDXC:Settings 2026-09-16 WHY:
 * Agentation persists its toolbar state with direct Storage calls inside a React effect, and a throw there unmounts the whole tool.
 * Writes that `forward` accepts (keys of a registered external store) take the metered path; every other direct write is still rejected.
 */
export function installBrowserGuard(forward: BrowserWriteForwarder): void {
  if (state.guarded || typeof window === 'undefined') return;
  area('local');
  area('session');
  state.guarded = true;
  const backendOf = (storage: unknown): BrowserBackend | undefined =>
    storage === state.areas.local ? 'local' : storage === state.areas.session ? 'session' : undefined;
  const reject = (operation: string, key?: string): never => {
    recordStorageEvent({ store: key ?? 'unknown', operation: 'unexpected', bytes: 0, reason: 'unregistered' });
    throw new ClientStorageError(
      'unregistered',
      key ?? '',
      `Direct browser storage ${operation} is forbidden. Register a store in packages/client-storage/catalog.ts.`
    );
  };
  Storage.prototype.setItem = function (this: Storage, key: string, value: string) {
    const backend = backendOf(this);
    if (backend && forward(backend, String(key), String(value))) return;
    reject('write', String(key));
  };
  Storage.prototype.removeItem = function (this: Storage, key: string) {
    const backend = backendOf(this);
    if (backend && forward(backend, String(key), null)) return;
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
