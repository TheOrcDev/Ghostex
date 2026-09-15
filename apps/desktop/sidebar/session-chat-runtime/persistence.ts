import { initializeClientStorage, managedStore } from '@/packages/client-storage';
import type { GxserverReadSessionChatResult } from '@/packages/shared/session-chat';

const MAX_RECORD_BYTES = 2 * 1024 * 1024;
const MAX_AGE_MS = 7 * 24 * 60 * 60 * 1_000;

export interface StoredSnapshot {
  key: string;
  savedAt: number;
  requestedWindow?: number;
  snapshot: GxserverReadSessionChatResult;
}

const storage = managedStore('chatSnapshots');

export async function readPersistedSessionChat(key: string): Promise<StoredSnapshot | undefined> {
  try {
    await initializeClientStorage();
    const stored = storage.get(key) as unknown as StoredSnapshot | undefined;
    return stored && Date.now() - stored.savedAt < MAX_AGE_MS ? stored : undefined;
  } catch {
    return undefined;
  }
}

export async function persistSessionChat(
  key: string,
  snapshot: GxserverReadSessionChatResult,
  savedAt: number,
  requestedWindow: number
): Promise<void> {
  try {
    await initializeClientStorage();
    await storage.update(key, (value) => {
      const previous = value as unknown as StoredSnapshot | undefined;
      if ((previous?.savedAt ?? 0) > savedAt) return value;
      // Keep the server's pagination cursor intact; oversized snapshots are disposable.
      if (JSON.stringify(snapshot).length * 2 > MAX_RECORD_BYTES) return undefined;
      return { key, savedAt, requestedWindow, snapshot } as unknown as NonNullable<typeof value>;
    });
  } catch {
    // The managed inspector records cache failures; the live stream remains authoritative.
  }
}
