import { storageScope } from '@/packages/client-storage';
import { GPUI_REMOTE_LAST_SEEN_PRESENTATIONS_STORAGE_KEY } from '../constants';
import { isPresentationSnapshot } from './remote-presentation';
import type { GxserverPresentationSnapshot } from '@/packages/shared/gxserver-protocol';

const clientStorage = storageScope(["remotePresentations"]);

const machineKeyPrefix = `${GPUI_REMOTE_LAST_SEEN_PRESENTATIONS_STORAGE_KEY}:machine:`;

/**
 * CDXC:RemoteMachines 2026-09-11 WHY:
 * Persist sanitized last-seen sidebar snapshots per machine so one remote update does not serialize every machine or overwrite another window's unrelated snapshots.
 * Discover stored machines only at startup; migration only fills missing keys so an older legacy copy cannot replace a newer per-machine snapshot.
 */
export class GpuiRemoteLastSeenStore {
  private pending = new Map<
    string,
    { snapshot: GxserverPresentationSnapshot | null; migrateOnly?: boolean; serialized?: string }
  >();
  private migrating = false;

  read(): Map<string, GxserverPresentationSnapshot> {
    const next = new Map<string, GxserverPresentationSnapshot>();
    try {
      const legacy = clientStorage.getItem(GPUI_REMOTE_LAST_SEEN_PRESENTATIONS_STORAGE_KEY);
      if (legacy !== null) {
        this.migrating = true;
        try {
          const raw: unknown = JSON.parse(legacy);
          if (raw && typeof raw === 'object' && !Array.isArray(raw)) {
            for (const [machineId, snapshot] of Object.entries(raw)) {
              if (machineId.trim() && isPresentationSnapshot(snapshot)) {
                next.set(machineId, snapshot);
                this.pending.set(machineId, { snapshot, migrateOnly: true });
              }
            }
          }
        } catch {
          // A malformed legacy entry must not hide valid per-machine entries.
        }
      }
      for (let index = 0; index < clientStorage.length; index += 1) {
        const key = clientStorage.key(index);
        if (!key?.startsWith(machineKeyPrefix)) continue;
        try {
          const machineId = decodeURIComponent(key.slice(machineKeyPrefix.length));
          const snapshot: unknown = JSON.parse(clientStorage.getItem(key) ?? 'null');
          if (machineId.trim() && isPresentationSnapshot(snapshot)) {
            next.set(machineId, snapshot);
            this.pending.delete(machineId);
          }
        } catch {
          // One malformed machine must not discard the other offline snapshots.
        }
      }
    } catch {
      // Storage can be unavailable during early CEF bootstrap.
    }
    return next;
  }

  queue(machineId: string, snapshot: GxserverPresentationSnapshot | null): void {
    this.pending.set(machineId, { snapshot });
  }

  get hasPending(): boolean {
    return this.pending.size > 0 || this.migrating;
  }

  flush(): void {
    for (const [machineId, update] of this.pending) {
      try {
        const key = `${machineKeyPrefix}${encodeURIComponent(machineId)}`;
        if (update.snapshot === null) {
          clientStorage.removeItem(key);
        } else if (!update.migrateOnly || clientStorage.getItem(key) === null) {
          update.serialized ??= JSON.stringify(update.snapshot);
          clientStorage.setItem(key, update.serialized);
        }
        this.pending.delete(machineId);
      } catch {
        // Keep failed writes queued so the next batch retries even without a new snapshot.
      }
    }
    if (this.migrating && this.pending.size === 0) {
      try {
        clientStorage.removeItem(GPUI_REMOTE_LAST_SEEN_PRESENTATIONS_STORAGE_KEY);
        this.migrating = false;
      } catch {
        // Keep the legacy copy until every migrated machine has been stored.
      }
    }
  }
}
