/**
 * The React Native composer under the GPUI transcript.
 *
 * The chat core owns the draft, exactly as on the desktop: every edit goes to it (`setDraft`), and
 * the text it holds comes back in each `summary`. The field shows the core's text whenever the
 * core changed it on its own (a restored draft, a send that failed and put the text back, the clear
 * after a confirmed send, a slash command completed in place).
 *
 * Send is optimistic: the field empties at once. Until the core has cleared its own copy, a summary
 * still carrying the sent text is ignored; once it has, the same text coming back means the send
 * failed and it is shown again.
 */
import { useEffect, useRef, useState } from 'react';
import { Pressable, StyleSheet, Text, TextInput, View } from 'react-native';

import { command } from '../../modules/gpui-view/src';
import type { ComposerSummary, SendMode, SendOutcome } from './types';

type Props = {
  summary: ComposerSummary | null;
  lastSend: { id: number; outcome: SendOutcome; reason: string | null } | null;
  bottomInset: number;
};

type PendingSend = { id: number; text: string; coreCleared: boolean };

let nextSendId = 1;

export default function ChatComposer({ summary, lastSend, bottomInset }: Props) {
  const [text, setText] = useState('');
  // The text the core was last told about, so its echo is not mistaken for its own change.
  const pushed = useRef('');
  const pending = useRef<PendingSend | null>(null);

  // The core's draft, when it changed it itself.
  useEffect(() => {
    if (!summary) return;
    const draft = summary.draft;
    const sent = pending.current;
    if (sent && !sent.coreCleared) {
      if (draft === '') sent.coreCleared = true;
      if (draft === '' || draft === sent.text) return;
    }
    if (draft !== pushed.current) {
      pushed.current = draft;
      setText(draft);
    }
  }, [summary]);

  // A send the chat refused before it left: put the text back.
  useEffect(() => {
    const sent = pending.current;
    if (!lastSend || !sent || lastSend.id !== sent.id) return;
    if (lastSend.outcome !== 'sent' && lastSend.outcome !== 'commandCompleted') {
      pending.current = null;
      pushed.current = sent.text;
      setText(sent.text);
    }
  }, [lastSend]);

  const onChangeText = (value: string) => {
    setText(value);
    pushed.current = value;
    command({ type: 'setDraft', text: value });
  };

  const ready = summary?.composerReady ?? false;
  const working = summary?.working ?? false;
  const canSend = ready && text.trim().length > 0 && !summary?.pendingSend;

  const submit = (mode: SendMode) => {
    if (!canSend) return;
    const id = nextSendId++;
    pending.current = { id, text, coreCleared: false };
    pushed.current = '';
    setText('');
    command({ type: 'send', text, mode, id });
  };

  const notice =
    summary?.error ??
    summary?.operationError ??
    summary?.sendBlockedReason ??
    (summary?.questionCardVisible
      ? summary.promptKind === 'approval'
        ? `Waiting for approval: ${summary.approvalAsk || 'answer on the desktop'}`
        : 'The agent is asking a question (answer on the desktop for now).'
      : null);

  return (
    <View style={[styles.root, { paddingBottom: Math.max(bottomInset, 8) }]}>
      {(notice || working || (summary?.queueCount ?? 0) > 0) && (
        <View style={styles.statusRow}>
          {working && <Text style={styles.working}>Working…</Text>}
          {(summary?.queueCount ?? 0) > 0 && (
            <Text style={styles.queued}>{summary!.queueCount} queued</Text>
          )}
          {notice && (
            <Text style={styles.notice} numberOfLines={2}>
              {notice}
            </Text>
          )}
        </View>
      )}
      <View style={styles.row}>
        <TextInput
          accessibilityLabel="composer-input"
          style={styles.input}
          value={text}
          onChangeText={onChangeText}
          onBlur={() => command({ type: 'saveDraft' })}
          placeholder={summary?.placeholder || (ready ? 'Message' : 'Loading…')}
          placeholderTextColor="#6c6c6c"
          multiline
          editable={summary !== null}
        />
        {working && (
          <Pressable
            accessibilityLabel="composer-stop"
            accessibilityRole="button"
            onPress={() => command({ type: 'action', action: { type: 'interrupt' } })}
            style={({ pressed }) => [styles.stop, pressed && styles.pressed]}
          >
            <Text style={styles.buttonLabel}>Stop</Text>
          </Pressable>
        )}
        <Pressable
          accessibilityLabel="composer-send"
          accessibilityRole="button"
          accessibilityHint={working && summary?.canQueue ? 'Long press to queue' : undefined}
          onPress={() => submit('send')}
          onLongPress={() => (summary?.canQueue ? submit('queue') : undefined)}
          style={({ pressed }) => [styles.send, !canSend && styles.disabled, pressed && styles.pressed]}
        >
          <Text style={styles.buttonLabel}>Send</Text>
        </Pressable>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  root: {
    paddingHorizontal: 10,
    paddingTop: 6,
    backgroundColor: '#1f1f1f',
    borderTopWidth: StyleSheet.hairlineWidth,
    borderTopColor: '#333',
  },
  statusRow: { flexDirection: 'row', alignItems: 'center', gap: 10, paddingBottom: 6, paddingHorizontal: 4 },
  working: { color: '#8ab4ff', fontSize: 13, fontWeight: '600' },
  queued: { color: '#c9c9c9', fontSize: 13 },
  notice: { flex: 1, color: '#e0a060', fontSize: 13 },
  row: { flexDirection: 'row', alignItems: 'flex-end', gap: 8 },
  input: {
    flex: 1,
    minHeight: 40,
    maxHeight: 140,
    paddingHorizontal: 12,
    paddingVertical: 9,
    borderRadius: 20,
    backgroundColor: '#2b2d31',
    color: '#eeeeee',
    fontSize: 15,
  },
  send: {
    height: 40,
    paddingHorizontal: 16,
    borderRadius: 20,
    backgroundColor: '#3d6ee0',
    justifyContent: 'center',
  },
  stop: {
    height: 40,
    paddingHorizontal: 14,
    borderRadius: 20,
    backgroundColor: '#5a2d2d',
    justifyContent: 'center',
  },
  disabled: { opacity: 0.45 },
  pressed: { opacity: 0.7 },
  buttonLabel: { color: '#fff', fontSize: 15, fontWeight: '600' },
});
