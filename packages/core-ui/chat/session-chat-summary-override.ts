import { storageScope, type ScopedStorage } from '@/packages/client-storage';
// Per-session Summary Mode state. Summary mode is opt-in for each chat and
// survives page reloads without becoming a global presentation default.

import { detectghostexHotkeyPlatform } from '../../shared/ghostex-hotkeys';

const clientStorage = storageScope(["summary"]);

const STORAGE_PREFIX = 'ghostex.sessionChat.summary.';

export function sessionChatSummaryToggleHotkey(): string {
  return detectghostexHotkeyPlatform() === 'mac' ? 'cmd+ctrl+shift+s' : 'cmd+alt+shift+s';
}

function storage(): ScopedStorage | null {
  try {
    return clientStorage;
  } catch {
    // Storage disabled by the embedder: the toggle still works per mount.
    return null;
  }
}

export function readStoredSessionChatSummary(sessionKey: string | null | undefined): boolean {
  if (!sessionKey) {
    return false;
  }
  try {
    return storage()?.getItem(`${STORAGE_PREFIX}${sessionKey}`) === '1';
  } catch {
    return false;
  }
}

export function writeStoredSessionChatSummary(sessionKey: string | null | undefined, summaryMode: boolean): void {
  if (!sessionKey) {
    return;
  }
  try {
    storage()?.setItem(`${STORAGE_PREFIX}${sessionKey}`, summaryMode ? '1' : '0');
  } catch {
    // Quota/private-mode failures must not break the toggle.
  }
}
