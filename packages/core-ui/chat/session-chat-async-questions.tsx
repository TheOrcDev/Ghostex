import { storageScope } from '@/packages/client-storage';
import { IconChevronDown, IconChevronLeft, IconChevronRight } from '@tabler/icons-react';
import { useId, useMemo, useRef, useState } from 'react';
import { Button } from '@/packages/components/ui/button';
import type { SessionChatMessage, SessionChatTheme } from '@/packages/shared/session-chat';
import { SessionChatChoiceRows } from './session-chat-choice-rows';
import { SessionQuestionIndicator } from '../session-question-indicator';
import { pendingSessionChatAsyncQuestions } from './session-chat-async-questions-state';
import { useSessionChatQuestionDrafts } from './session-chat-question-drafts';
import { SessionChatAnswerInput } from './session-chat-answer-input';
import type { SaveSessionChatImage } from './session-chat-image-attachments';
import './session-chat-async-questions.css';

const clientStorage = storageScope(["retiredQuestions"]);

/**
 * CDXC:SessionChat 2026-09-12 DECISION:
 * User: Codex questions asked while it is still working appear in a new component above the chat composer.
 * The composer keeps its draft and remains usable; choosing a suggested answer requires an explicit send.
 */
export function SessionChatAsyncQuestions({
  messages,
  canSend,
  working,
  onSend,
  onDismiss,
  sessionKey,
  onPasteImage,
  theme,
}: {
  messages: readonly SessionChatMessage[];
  canSend: boolean;
  working: boolean;
  onSend: (questionId: string, text: string) => Promise<void>;
  onDismiss: (questionId: string) => Promise<void>;
  sessionKey?: string;
  onPasteImage?: SaveSessionChatImage;
  theme?: SessionChatTheme;
}) {
  const storageKey = sessionKey ? `ghostex:async-questions:${sessionKey}` : null;
  const [retired, setRetired] = useState<string[]>(() => {
    if (!storageKey) return [];
    try {
      const value: unknown = JSON.parse(clientStorage.getItem(storageKey) ?? '[]');
      return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : [];
    } catch {
      return [];
    }
  });
  const pending = useMemo(
    () => pendingSessionChatAsyncQuestions(messages).filter((question) => !retired.includes(question.key)),
    [messages, retired]
  );
  const [activeKey, setActiveKey] = useState<string | null>(null);
  const { drafts, saveDrafts, updateDraft, clearDrafts, saveError } = useSessionChatQuestionDrafts(sessionKey, 'async');
  const [savingImages, setSavingImages] = useState(false);
  const [collapsed, setCollapsed] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const submittingRef = useRef(false);
  const panelId = useId();
  const index = Math.max(
    0,
    pending.findIndex((question) => question.key === activeKey)
  );
  const question = pending[index];
  if (!question) return null;
  const draft = drafts[question.key] ?? { indices: [0], other: '' };
  const answer = draft.other.trim() || question.options?.[draft.indices[0] ?? 0] || '';
  const disabled = !canSend || submitting || savingImages;

  const retire = (key: string): void => {
    if (drafts[key]) clearDrafts({ [key]: drafts[key] });
    setRetired((current) => {
      const next = [...current, key].slice(-1000);
      if (storageKey) {
        try {
          clientStorage.setItem(storageKey, JSON.stringify(next));
        } catch {
          /* The mounted view still retains accepted answers. */
        }
      }
      return next;
    });
  };
  const submit = async (): Promise<void> => {
    if (disabled || submittingRef.current || !answer.trim()) return;
    submittingRef.current = true;
    setSubmitting(true);
    setError(null);
    try {
      await onSend(question.key, answer.trim());
      retire(question.key);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Could not send your answer. Please try again.');
    } finally {
      submittingRef.current = false;
      setSubmitting(false);
    }
  };
  const skip = async (): Promise<void> => {
    if (disabled || submittingRef.current) return;
    submittingRef.current = true;
    setSubmitting(true);
    setError(null);
    try {
      await onDismiss(question.key);
      retire(question.key);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Could not skip this question. Please try again.');
    } finally {
      submittingRef.current = false;
      setSubmitting(false);
    }
  };

  return (
    <section
      className='ghostex-chat-async-questions'
      aria-label='Questions from Codex'
      data-chat-async-questions='true'
    >
      <span className='sr-only' role='status'>
        {pending.length} unanswered question{pending.length === 1 ? '' : 's'} from Codex.
        {working ? ' The agent is still working.' : ''}
      </span>
      <button
        className='ghostex-chat-async-questions-header'
        data-slot='async-questions-header'
        type='button'
        aria-expanded={!collapsed}
        aria-controls={panelId}
        onClick={() => setCollapsed((value) => !value)}
      >
        <SessionQuestionIndicator working={working} />
        <span className='font-medium'>Question{pending.length > 1 ? 's' : ''} from Codex</span>
        {/* CDXC:SessionChat 2026-09-14 DECISION: User: remove the idle "Reply when ready" label from the Codex questions card. */}
        <span className='min-w-0 flex-1 text-muted-foreground'>{working ? 'Still working' : null}</span>
        <span className='text-muted-foreground'>
          {index + 1}/{pending.length}
        </span>
        {collapsed ? <IconChevronRight size={16} /> : <IconChevronDown size={16} />}
      </button>
      {!collapsed ? (
        <div key={question.key} id={panelId} className='ghostex-chat-async-questions-body'>
          <p className='whitespace-pre-wrap' id={`${panelId}-question`}>
            {question.title}
          </p>
          {question.options?.length ? (
            <SessionChatChoiceRows
              options={question.options.map((label) => ({ label }))}
              selected={draft.other.trim() ? [] : draft.indices}
              readOnly={disabled}
              onSelect={(selected) => saveDrafts({ ...drafts, [question.key]: { indices: [selected], other: '' } })}
            />
          ) : null}
          <SessionChatAnswerInput
            className='ghostex-chat-async-questions-answer'
            theme={theme}
            aria-label='Your answer'
            aria-describedby={`${panelId}-question`}
            placeholder={question.options?.length ? 'Or write your own answer…' : 'Write your answer…'}
            disabled={submitting}
            value={draft.other}
            onPasteImage={onPasteImage}
            onPendingChange={setSavingImages}
            onUpdate={(update) =>
              updateDraft(question.key, (current) => ({ ...current, other: update(current.other) }))
            }
            // CDXC:SessionChat 2026-09-12 DECISION: User: Enter sends a question answer; Shift+Enter inserts a newline.
            onKeyDown={(event) => {
              if (event.key !== 'Enter' || event.shiftKey || event.isComposing || event.keyCode === 229)
                return;
              event.preventDefault();
              if (!event.repeat) void submit();
            }}
          />
          {error || saveError ? (
            <p className='text-destructive' role='alert'>
              {error || saveError}
            </p>
          ) : null}
          {!canSend ? (
            <p className='text-muted-foreground' role='status'>
              Answers are unavailable while this chat is read-only or disconnected.
            </p>
          ) : null}
          <div className='ghostex-chat-async-questions-actions'>
            {pending.length > 1 ? (
              <>
                <Button
                  aria-label='Previous question'
                  size='icon-sm'
                  variant='ghost'
                  disabled={submitting || savingImages || index === 0}
                  onClick={() => {
                    setActiveKey(pending[index - 1]!.key);
                    setError(null);
                  }}
                >
                  <IconChevronLeft size={16} />
                </Button>
                <Button
                  aria-label='Next question'
                  size='icon-sm'
                  variant='ghost'
                  disabled={submitting || savingImages || index === pending.length - 1}
                  onClick={() => {
                    setActiveKey(pending[index + 1]!.key);
                    setError(null);
                  }}
                >
                  <IconChevronRight size={16} />
                </Button>
              </>
            ) : null}
            <Button className='ml-auto' size='sm' variant='ghost' disabled={disabled} onClick={() => void skip()}>
              Skip
            </Button>
            <Button
              size='sm'
              variant='outline'
              data-chat-answer-control=''
              disabled={disabled || !answer.trim()}
              onClick={() => void submit()}
            >
              {submitting ? 'Sending…' : 'Send answer'}
            </Button>
          </div>
        </div>
      ) : null}
    </section>
  );
}
