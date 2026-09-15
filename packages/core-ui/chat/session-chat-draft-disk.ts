import { atomicStorage, flushClientStorage, initializeClientStorage, storageScope } from '@/packages/client-storage';
import type { PendingDraft } from './session-chat-draft-outbox';

const storage = storageScope(['draftOutbox']);
function key(draft: PendingDraft): string {
  return `ghostex.sessionChat.outbox.${draft.sessionKey}:${draft.version.draftId}:${draft.version.revision}`;
}

/** Recovery receipts and checkpoint removal commit together, using the latest values from every page. */
export function withDraftStorageLock(action: () => void): Promise<void> {
  return atomicStorage(['recovery', 'recoveryDismissed'], action);
}
export async function saveDraftToDisk(draft: PendingDraft): Promise<void> {
  await initializeClientStorage();
  storage.setItem(key(draft), JSON.stringify(draft));
  await flushClientStorage(['drafts', 'recovery', 'recoveryDismissed', 'draftOutbox', 'composerSelection', 'questionDrafts']);
}
export async function readDraftsFromDisk(): Promise<PendingDraft[]> {
  await initializeClientStorage();
  return storage.keys().map((key) => JSON.parse(storage.getItem(key)!) as PendingDraft);
}
export async function removeDraftFromDisk(draft: PendingDraft): Promise<void> {
  storage.removeItem(key(draft));
  await flushClientStorage(['drafts', 'recovery', 'recoveryDismissed', 'draftOutbox', 'composerSelection', 'questionDrafts']);
}
