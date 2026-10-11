import { storageCatalog, type StoreId } from './catalog';
import {
  diagnosticPageId,
  localStorageActivity,
  mergeStorageEvents,
  notifyStorage,
  onLocalStorageActivity,
  type StorageActivity,
} from './diagnostics';

export type PendingUsage = { store: StoreId; count: number; bytes: number };
type PeerStatus = { page: string; activity: StorageActivity; pending: PendingUsage[] };
type PeerMessage =
  | { type: 'status'; status: PeerStatus }
  | { type: 'request-status' | 'retry' | 'bye'; page: string }
  | { type: 'clear-cache'; page: string; store: StoreId };
const peers = new Map<string, { receivedAt: number; status: PeerStatus }>();
let publish: (() => void) | undefined;
let send: ((message: PeerMessage) => void) | undefined;
let requestedAt = 0;

export function connectStoragePeers(
  channel: BroadcastChannel,
  pending: () => PendingUsage[],
  retry: () => void,
  clearCache: (store: StoreId) => Promise<void>
): void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  send = (message) => channel.postMessage(message);
  const status = () => {
    if (timer) clearTimeout(timer);
    timer = undefined;
    send?.({
      type: 'status',
      status: { page: diagnosticPageId, activity: localStorageActivity(), pending: pending() },
    });
  };
  publish = () => {
    if (!timer) timer = setTimeout(status, 500);
  };
  onLocalStorageActivity(publish);
  channel.addEventListener('message', (event: MessageEvent<PeerMessage>) => {
    const message = event.data;
    if (!message || Array.isArray(message)) return;
    switch (message.type) {
      case 'status': {
        const incoming = message.status;
        if (incoming.page === diagnosticPageId) return;
        peers.delete(incoming.page);
        peers.set(incoming.page, { receivedAt: Date.now(), status: incoming });
        while (peers.size > 64) peers.delete(peers.keys().next().value!);
        mergeStorageEvents([...incoming.activity.events, ...incoming.activity.failures]);
        notifyStorage();
        break;
      }
      case 'request-status':
        publish?.();
        break;
      case 'bye':
        peers.delete(message.page);
        notifyStorage();
        break;
      case 'retry':
        retry();
        break;
      case 'clear-cache':
        if (storageCatalog[message.store]?.policy === 'cache') void clearCache(message.store).catch(() => {});
        break;
    }
  });
  window.addEventListener('pagehide', () => send?.({ type: 'bye', page: diagnosticPageId }));
  window.addEventListener('pageshow', status);
  setInterval(status, 30_000);
  requestStoragePeerStatus();
  status();
}

export function publishStorageStatus(): void {
  publish?.();
}
export function requestStoragePeerStatus(): void {
  if (Date.now() - requestedAt < 5_000 || !send) return;
  requestedAt = Date.now();
  send({ type: 'request-status', page: diagnosticPageId });
}
export function retryStoragePeers(): void {
  send?.({ type: 'retry', page: diagnosticPageId });
}
export function clearStoragePeerCache(store: StoreId): void {
  send?.({ type: 'clear-cache', page: diagnosticPageId, store });
}
export function peerStorageUsage(store: string): { pending: number; pendingBytes: number; writesPerMinute: number } {
  const result = { pending: 0, pendingBytes: 0, writesPerMinute: 0 };
  const now = Date.now();
  const cutoff = Math.floor(now / 1_000) - 60;
  for (const [page, peer] of peers) {
    if (now - peer.receivedAt > 120_000) {
      peers.delete(page);
      continue;
    }
    const pending = peer.status.pending.find((entry) => entry.store === store);
    result.pending += pending?.count ?? 0;
    result.pendingBytes += pending?.bytes ?? 0;
    const buckets = peer.status.activity.writes.find(([id]) => id === store)?.[1] ?? [];
    result.writesPerMinute += buckets.reduce((sum, bucket) => sum + (bucket.second > cutoff ? bucket.count : 0), 0);
  }
  return result;
}
