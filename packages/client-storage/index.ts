export { managedStore, storageScope, type ScopedStorage } from './handles';
export { storageCatalog, type StoreId } from './catalog';
export {
  initializeClientStorage,
  flushClientStorage,
  atomicStorage,
  retryClientStorage,
  inspectClientStorage,
  clearDisposableStorage,
  storageFailure,
  isStoragePending,
  subscribeStorage,
  installBrowserGuard,
} from './service';
export { setStorageDiagnosticSink } from './diagnostics';
export type { StorageInspection, StoreUsage, StorageEvent } from './types';
