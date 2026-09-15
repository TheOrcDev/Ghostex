import { storageScope } from '@/packages/client-storage';
import type { SessionChatDraftVersion, SessionChatRecoveryDraft } from '@/packages/shared/session-chat-queue';
import { reportDraftStorageFailure } from './session-chat-draft-outbox';
import { SessionChatStorageIndex } from './session-chat-storage-index';
import {
  compactDraftRecoveryDismissals,
  draftRecoveryDismissalMarker,
  isDraftRecoveryDismissed,
} from './session-chat-draft-dismissals';

const clientStorage = storageScope(["recovery"]);

const PREFIX = 'ghostex.sessionChat.recovery.';
export type LocalRecoveryDraft = {
  sessionKey: string;
  text: string;
  updatedAt: number;
  version?: SessionChatDraftVersion;
  dismissed?: boolean;
};
const recoveryIndex = new SessionChatStorageIndex<LocalRecoveryDraft>(
  'recovery',
  PREFIX,
  (raw) => {
    const entry = JSON.parse(raw) as LocalRecoveryDraft;
    return entry && typeof entry.text === 'string' && !entry.dismissed ? entry : null;
  },
  (entry) => entry.sessionKey
);
function identity(entry: LocalRecoveryDraft): string {
  return `${entry.sessionKey}:${entry.version ? `${entry.version.draftId}:${entry.version.revision}` : entry.updatedAt}`;
}
export function preserveDraftRevision(entry: LocalRecoveryDraft): void {
  if (entry.text === '') return;
  try {
    const name = PREFIX + identity(entry);
    if (clientStorage.getItem(name) === null && !isDraftRecoveryDismissed(entry.sessionKey, entry.version)) {
      recoveryIndex.set(name, entry);
    }
  } catch {
    reportDraftStorageFailure(entry.sessionKey);
  }
}
export function retireDraftRecovery(sessionKey: string, receipts: readonly SessionChatDraftVersion[]): void {
  if (receipts.length === 0) return;
  const consumed = new Map<string, number>();
  for (const receipt of receipts)
    consumed.set(receipt.draftId, Math.max(consumed.get(receipt.draftId) ?? 0, receipt.revision));
  for (const [id, entry] of recoveryDraftEntries(sessionKey)) {
    if (
      entry.sessionKey === sessionKey &&
      entry.version &&
      (consumed.get(entry.version.draftId) ?? 0) >= entry.version.revision
    )
      dismissDraftRecovery(id);
  }
}
export function recoveryDraftEntries(sessionKey?: string): [string, LocalRecoveryDraft][] {
  prepareDraftRecoveryStorage(sessionKey);
  try {
    return recoveryIndex.entries(sessionKey).map(([name, entry]) => [name.slice(PREFIX.length), entry]);
  } catch {
    /* Storage errors are reported by the editing path. */
  }
  return [];
}
export function dismissDraftRecovery(id: string): void {
  prepareDraftRecoveryStorage();
  const raw = clientStorage.getItem(PREFIX + id);
  if (!raw) return;
  const entry = JSON.parse(raw) as LocalRecoveryDraft | null;
  if (!entry || typeof entry.text !== 'string') return;
  clientStorage.setItem(PREFIX + id, draftRecoveryDismissalMarker(entry.sessionKey, entry.version));
  // Refresh this page's index too: storage events only notify other pages.
  recoveryIndex.refresh(PREFIX + id);
  void compactDraftRecoveryDismissals((name) => recoveryIndex.remove(name), PREFIX + id).catch(() =>
    reportDraftStorageFailure(entry.sessionKey)
  );
}
export function prepareDraftRecoveryStorage(sessionKey?: string): void {
  void compactDraftRecoveryDismissals((name) => recoveryIndex.remove(name)).catch(() => {
    if (sessionKey) reportDraftStorageFailure(sessionKey);
  });
}
export function importDraftRecovery(drafts: readonly SessionChatRecoveryDraft[] = [], prefix = ''): void {
  prepareDraftRecoveryStorage();
  for (const draft of drafts)
    preserveDraftRevision({
      sessionKey: `${prefix}${draft.projectId}:${draft.sessionId}`,
      text: draft.content,
      version: draft.version,
      updatedAt: Date.parse(draft.updatedAt),
    });
}
