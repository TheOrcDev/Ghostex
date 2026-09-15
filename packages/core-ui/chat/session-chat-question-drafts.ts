import { storageScope, storageFailure, subscribeStorage } from '@/packages/client-storage';
import { useCallback, useRef, useState, useSyncExternalStore } from 'react';
import { SessionChatStorageIndex } from './session-chat-storage-index';

const clientStorage = storageScope(["questionDrafts"]);

export interface SessionChatAnswerDraft {
  indices: number[];
  other: string;
}

type AnswerDrafts = Record<string, SessionChatAnswerDraft>;
const PREFIX = 'ghostex.sessionChat.questionDraft.';
const draftIndex = new SessionChatStorageIndex<AnswerDrafts>(
  'questionDrafts',
  PREFIX, decodeDrafts, () => '');

function decodeDrafts(raw: string): AnswerDrafts | null {
  try {
    const value = JSON.parse(raw) as AnswerDrafts;
    if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
    return Object.values(value).every(
      (answer) =>
        answer &&
        typeof answer.other === 'string' &&
        Array.isArray(answer.indices) &&
        answer.indices.every((index) => Number.isSafeInteger(index) && index >= 0)
    )
      ? value
      : null;
  } catch {
    return null;
  }
}

function readDrafts(key: string | null): AnswerDrafts {
  if (!key) return {};
  try {
    return decodeDrafts(clientStorage.getItem(key) ?? '{}') ?? {};
  } catch {
    return {};
  }
}

/**
 * CDXC:SessionChat 2026-09-15 DECISION:
 * User: answer text in question cards must survive session switches, reusing the composer's draft storage system.
 * Save each edit through the same local storage index, scoped to the session and question, and clear only after successful delivery or explicit dismissal.
 */
export function useSessionChatQuestionDrafts(sessionKey: string | undefined, promptKey: string) {
  const persistenceError = useSyncExternalStore(subscribeStorage, () => storageFailure('questionDrafts'), () => undefined);
  const scope = JSON.stringify([sessionKey, promptKey]);
  const key = sessionKey ? `${PREFIX}${scope}` : null;
  const [state, setState] = useState(() => ({ scope, drafts: readDrafts(key), error: '' }));
  if (state.scope !== scope) setState({ scope, drafts: readDrafts(key), error: '' });
  const stateRef = useRef(state);
  stateRef.current = state;

  const saveDrafts = useCallback(
    (drafts: AnswerDrafts): void => {
      let error = '';
      if (key) {
        try {
          if (Object.keys(drafts).length) draftIndex.set(key, drafts);
          else draftIndex.remove(key);
        } catch {
          error = 'Your answer could not be saved on this computer. Keep this view open until saving succeeds.';
        }
      }
      if (stateRef.current.scope === scope) {
        stateRef.current = { scope, drafts, error };
        setState(stateRef.current);
      }
    },
    [key, scope]
  );

  const clearDrafts = useCallback(
    (submitted: AnswerDrafts): void => {
      const remaining = key
        ? readDrafts(key)
        : { ...(stateRef.current.scope === scope ? stateRef.current.drafts : {}) };
      for (const [question, answer] of Object.entries(submitted)) {
        if (JSON.stringify(remaining[question]) === JSON.stringify(answer)) delete remaining[question];
      }
      if (key) {
        try {
          if (Object.keys(remaining).length) draftIndex.set(key, remaining);
          else draftIndex.remove(key);
        } catch {
          return;
        }
      }
      setState((current) => (current.scope === scope ? { scope, drafts: remaining, error: '' } : current));
    },
    [key, scope]
  );

  const updateDraft = useCallback(
    (question: string, update: (draft: SessionChatAnswerDraft) => SessionChatAnswerDraft): void => {
      const current =
        key && !stateRef.current.error
          ? readDrafts(key)
          : stateRef.current.scope === scope
            ? stateRef.current.drafts
            : {};
      saveDrafts({ ...current, [question]: update(current[question] ?? { indices: [], other: '' }) });
    },
    [key, scope, saveDrafts]
  );

  return { drafts: state.drafts, saveDrafts, updateDraft, clearDrafts, saveError: state.error || (persistenceError ? 'Your answer could not be saved on this computer. Keep this view open until saving succeeds.' : '') };
}
