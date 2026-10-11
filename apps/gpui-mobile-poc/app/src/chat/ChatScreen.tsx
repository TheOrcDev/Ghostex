/**
 * The chat screen: a header, the desktop's GPUI transcript (drawn by `libghostex_gpui_mobile.so`,
 * transcript-only), and the React Native composer. Same shape as the real app's
 * `apps/mobile/app/src/chat/native/NativeChatScreen.tsx`, with the transcript swapped for GPUI.
 *
 * The chat itself (its socket to gxserver, drafts, sends, the transcript) is the desktop's Rust
 * chat host running inside the library; this screen only forwards the composer's edits and sends
 * and performs what the transcript asks of the app (links, copy, toasts).
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { Linking, Pressable, StyleSheet, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

import {
  GpuiView,
  command,
  installKeyboardResize,
  launchOptions,
  setClipboardText,
  start,
  type GpuiEvent,
  type GpuiSurfaceType,
} from '../../modules/gpui-view/src';
import ChatComposer from './ChatComposer';
import { devEndpoint, type SessionRow } from './gxserver';
import SessionPicker from './SessionPicker';
import Toast, { useToast } from './Toast';
import type { ComposerSummary, SendOutcome } from './types';

type OpenSession = { key: string; title: string };

function initialSession(): OpenSession | null {
  const key = launchOptions().gpuiSession ?? devEndpoint().session;
  return key && key.includes(':') ? { key, title: key } : null;
}

let startedOnce = false;

export default function ChatScreen() {
  const insets = useSafeAreaInsets();
  const [session, setSession] = useState<OpenSession | null>(initialSession);
  const [summary, setSummary] = useState<ComposerSummary | null>(null);
  const [lastSend, setLastSend] = useState<{ id: number; outcome: SendOutcome; reason: string | null } | null>(
    null
  );
  const [pickerOpen, setPickerOpen] = useState(session === null);
  const [surfaceType, setSurfaceType] = useState<GpuiSurfaceType>('surface');
  const [perf, setPerf] = useState<{ gpu?: string; fps?: number; worst?: number }>({});
  const [showPerf, setShowPerf] = useState(false);
  const toast = useToast();
  const opened = useRef(session?.key ?? null);

  if (!startedOnce) {
    startedOnce = true;
    const endpoint = devEndpoint();
    const [projectId, sessionId] = session?.key.split(':') ?? ['', ''];
    const first = start({
      content: 'chat',
      baseUrl: endpoint.baseUrl,
      authToken: endpoint.authToken,
      projectId,
      sessionId,
    });
    // A reloaded JS bundle finds GPUI already running with its old chat: point it at ours.
    if (!first && session) {
      command({ type: 'openSession', projectId, sessionId });
    }
  }

  useEffect(() => {
    installKeyboardResize();
    command({ type: 'state' });
  }, []);

  const openSession = useCallback((row: SessionRow) => {
    const key = `${row.projectId}:${row.sessionId}`;
    setPickerOpen(false);
    if (opened.current === key) return;
    opened.current = key;
    setSummary(null);
    setSession({ key, title: row.title });
    command({ type: 'openSession', projectId: row.projectId, sessionId: row.sessionId });
  }, []);

  const onEvent = useCallback(
    (event: GpuiEvent) => {
      switch (event.type) {
        case 'summary':
        case 'state': {
          const next = event.summary as ComposerSummary | null | undefined;
          if (next && (!opened.current || next.session === opened.current)) setSummary(next);
          break;
        }
        case 'sendResult':
          setLastSend({
            id: Number(event.id),
            outcome: event.outcome as SendOutcome,
            reason: (event.reason as string | null) ?? null,
          });
          if (event.outcome === 'blocked' && event.reason) toast.show(String(event.reason), true);
          break;
        case 'hostAction': {
          const message = event.message as Record<string, unknown>;
          const url = typeof message.url === 'string' ? message.url : null;
          if (url && /^https?:/.test(url)) {
            Linking.openURL(url).catch(() => toast.show(`Could not open ${url}`, true));
          } else if (event.action === 'openFile' || event.action === 'openMarkdownLink') {
            toast.show(`Files open on the computer: ${String(message.path ?? url ?? '')}`);
          } else {
            toast.show(`Not on the phone yet: ${String(event.action)}`);
          }
          break;
        }
        case 'toast':
          toast.show(String(event.message), Boolean(event.error));
          break;
        case 'copy':
          setClipboardText(String(event.text));
          toast.show('Copied');
          break;
        case 'ready': {
          const gpu = event.gpu as { device?: string } | null;
          setPerf((previous) => ({ ...previous, gpu: gpu?.device ?? 'unknown' }));
          break;
        }
        case 'frameStats':
          setPerf((previous) => ({ ...previous, fps: Number(event.fps), worst: Number(event.worstGapMs) }));
          break;
        case 'error':
          toast.show(String(event.message), true);
          break;
      }
    },
    [toast]
  );

  const status = summary?.working ? 'working' : (summary?.status ?? 'loading');

  return (
    <View style={[styles.root, { paddingTop: insets.top, paddingLeft: insets.left, paddingRight: insets.right }]}>
      <View style={styles.header}>
        <Pressable
          accessibilityLabel="open-session-picker"
          onPress={() => setPickerOpen(true)}
          onLongPress={() => setShowPerf((value) => !value)}
          style={styles.titleButton}
        >
          <Text style={styles.title} numberOfLines={1}>
            {session?.title ?? 'Pick a session'}
          </Text>
          <Text style={styles.subtitle} numberOfLines={1}>
            {session ? `${status} · ${session.key}` : 'Tap to choose'}
          </Text>
        </Pressable>
        <Pressable
          accessibilityLabel="toggle-surface-type"
          onPress={() => setSurfaceType((type) => (type === 'surface' ? 'texture' : 'surface'))}
          hitSlop={8}
        >
          <Text style={styles.headerButton}>{surfaceType === 'surface' ? 'SV' : 'TV'}</Text>
        </Pressable>
      </View>
      {showPerf && (
        <Text style={styles.perf} numberOfLines={1}>
          {perf.gpu ?? '?'} · {perf.fps?.toFixed(1) ?? '-'} fps · worst {perf.worst?.toFixed(0) ?? '-'} ms
        </Text>
      )}
      <GpuiView style={styles.transcript} surfaceType={surfaceType} onEvent={onEvent} />
      <ChatComposer summary={session ? summary : null} lastSend={lastSend} bottomInset={insets.bottom} />
      <Toast state={toast.state} bottom={insets.bottom + 90} />
      <SessionPicker
        visible={pickerOpen}
        current={session?.key ?? null}
        onPick={openSession}
        onClose={() => setPickerOpen(false)}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#181818' },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 12,
    paddingHorizontal: 14,
    paddingVertical: 8,
    borderBottomWidth: StyleSheet.hairlineWidth,
    borderBottomColor: '#2e2e2e',
  },
  titleButton: { flex: 1 },
  title: { color: '#eee', fontSize: 16, fontWeight: '600' },
  subtitle: { color: '#8a8a8a', fontSize: 12, marginTop: 2 },
  headerButton: { color: '#8ab4ff', fontSize: 13, fontWeight: '600' },
  perf: { color: '#8a8a8a', fontSize: 11, paddingHorizontal: 14, paddingVertical: 2 },
  transcript: { flex: 1 },
});
