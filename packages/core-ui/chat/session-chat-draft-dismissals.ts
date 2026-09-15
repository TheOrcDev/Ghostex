import { storageScope } from '@/packages/client-storage';
import type { SessionChatDraftVersion } from '@/packages/shared/session-chat-queue';
import { withDraftStorageLock } from './session-chat-draft-disk';

const clientStorage = storageScope(["recovery","recoveryDismissed"]);

const RECOVERY_PREFIX = 'ghostex.sessionChat.recovery.';
const PREFIX = 'ghostex.sessionChat.recoveryDismissed.';
type Range = [number, number];
type Marker = [string, string, number];
const pending = new Set<string>();
let scanned = false;
let running: Promise<void> | undefined;

function key(sessionKey: string, draftId: string): string {
  return PREFIX + JSON.stringify([sessionKey, draftId]);
}

export function isDraftRecoveryDismissed(sessionKey: string, version?: SessionChatDraftVersion): boolean {
  if (!version) return false;
  const ranges: Range[] = JSON.parse(clientStorage.getItem(key(sessionKey, version.draftId)) ?? '[]');
  return ranges.some(([start, end]) => start <= version.revision && version.revision <= end);
}

export function draftRecoveryDismissalMarker(sessionKey: string, version?: SessionChatDraftVersion): string {
  return JSON.stringify(version ? [sessionKey, version.draftId, version.revision] : null);
}

function marker(raw: string): Marker | null {
  const entry = JSON.parse(raw);
  if (Array.isArray(entry)) {
    return typeof entry[0] === 'string' && typeof entry[1] === 'string' && Number.isSafeInteger(entry[2])
      ? (entry as Marker)
      : null;
  }
  return entry?.dismissed && entry.version ? [entry.sessionKey, entry.version.draftId, entry.version.revision] : null;
}

/**
 * CDXC:Drafts 2026-09-13 WHY:
 * Over 16,000 per-revision dismissal objects filled Chromium's localStorage quota and made ordinary typing fail to save.
 * Compact exact dismissed revisions into ranges per draft, preserving gaps and every active recovery checkpoint.
 * Commit the ranges before removing their old markers; shrink legacy markers first so migration works at full quota.
 * An IndexedDB write transaction prevents simultaneous pages from overwriting each other's dismissal ranges, including HTTP clients without Web Locks.
 */
export function compactDraftRecoveryDismissals(remove: (key: string) => void, name?: string): Promise<void> {
  if (name) pending.add(name);
  if (running) return running;
  if (scanned && pending.size === 0) return Promise.resolve();
  const processed = new Set<string>();
  running = withDraftStorageLock(() => {
    if (!scanned) {
      for (const name of clientStorage.keys()) {
        if (name.startsWith(RECOVERY_PREFIX)) pending.add(name);
      }
      scanned = true;
    }
    const groups = new Map<string, { names: string[]; revisions: Range[] }>();
    for (const name of pending) {
      processed.add(name);
      const raw = clientStorage.getItem(name);
      if (raw === null) continue;
      const entry = marker(raw);
      if (!entry) continue;
      const [sessionKey, draftId, revision] = entry;
      const compact = JSON.stringify(entry);
      if (compact.length < raw.length) clientStorage.setItem(name, compact);
      const groupKey = key(sessionKey, draftId);
      let group = groups.get(groupKey);
      if (!group) groups.set(groupKey, (group = { names: [], revisions: [] }));
      group.names.push(name);
      group.revisions.push([revision, revision]);
    }
    for (const [groupKey, group] of groups) {
      const previous: Range[] = JSON.parse(clientStorage.getItem(groupKey) ?? '[]');
      const merged: Range[] = [];
      for (const [start, end] of [...previous, ...group.revisions].sort((a, b) => a[0] - b[0])) {
        const last = merged[merged.length - 1];
        if (last && start <= last[1] + 1) last[1] = Math.max(last[1], end);
        else merged.push([start, end]);
      }
      clientStorage.setItem(groupKey, JSON.stringify(merged));
      for (const name of group.names) remove(name);
    }
  }).then(() => {
    for (const name of processed) pending.delete(name);
  }).catch((error: unknown) => {
    scanned = false;
    throw error;
  }).finally(() => {
    running = undefined;
  });
  return running;
}
