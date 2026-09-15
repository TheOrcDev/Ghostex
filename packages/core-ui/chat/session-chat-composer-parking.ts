import { storageScope, isStoragePending } from '@/packages/client-storage';

const clientStorage = storageScope(["composerSelection"]);
interface ParkedComposerSelection {
  text: string;
  start: number;
  end: number;
}
const prefix = 'ghostex.sessionChat.composerSelection.';

export function saveSessionChatComposerSelection(sessionKey: string, state: ParkedComposerSelection): void {
  const serialized = JSON.stringify(state);
  clientStorage.setItem(prefix + sessionKey, serialized);

}
export function readSessionChatComposerSelection(
  sessionKey: string | undefined,
  text: string
): ParkedComposerSelection | undefined {
  if (!sessionKey) return undefined;
  try {
    const state = JSON.parse(clientStorage.getItem(prefix + sessionKey) ?? 'null') as ParkedComposerSelection | null;
    if (state?.text !== text || !Number.isInteger(state.start) || !Number.isInteger(state.end)) return undefined;
    return {
      text,
      start: Math.max(0, Math.min(text.length, state.start)),
      end: Math.max(0, Math.min(text.length, state.end)),
    };
  } catch {
    return undefined;
  }
}

export function isSessionChatComposerSelectionDurable(sessionKey: string): boolean {
  return !isStoragePending('composerSelection', prefix + sessionKey);
}
