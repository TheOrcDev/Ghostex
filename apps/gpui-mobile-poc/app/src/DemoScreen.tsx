/**
 * The screen: dev toolbar, the GPUI view filling the space between, the composer at the bottom.
 * Mirrors the real chat screen's shape (apps/mobile/app/src/chat/native/NativeChatScreen.tsx).
 * The React root is shrunk above the software keyboard (`installKeyboardResize`, because an
 * edge-to-edge window ignores `adjustResize`), so the GPUI view shrinks by plain flex layout and
 * the composer stays above the keyboard.
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { StyleSheet, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

import {
  GpuiView,
  command,
  installKeyboardResize,
  start,
  type GpuiEvent,
  type GpuiSurfaceType,
} from '../modules/gpui-view/src';
import Composer from './Composer';
import DevToolbar, { type DevStats } from './DevToolbar';

const INITIAL_STATS: DevStats = {
  events: 0,
  taps: 0,
  rows: 0,
  fps: null,
  worstGapMs: null,
  rttMs: null,
  viewport: null,
  gpu: null,
  lastEvent: '',
};

export default function DemoScreen() {
  const insets = useSafeAreaInsets();
  const [surfaceType, setSurfaceType] = useState<GpuiSurfaceType>('surface');
  const [stats, setStats] = useState<DevStats>(INITIAL_STATS);
  const started = useRef(false);

  if (!started.current) {
    started.current = true;
    start({ content: 'demo', demoRows: 300 });
  }

  useEffect(() => {
    installKeyboardResize();
    command({ type: 'state' });
  }, []);

  const onEvent = useCallback((event: GpuiEvent) => {
    setStats((previous) => {
      const next: DevStats = { ...previous, events: previous.events + 1, lastEvent: event.type };
      switch (event.type) {
        case 'rowTapped':
          next.taps = previous.taps + 1;
          break;
        case 'appended':
        case 'state':
          next.rows = Number(event.rows ?? previous.rows);
          break;
        case 'frameStats':
          next.fps = Number(event.fps);
          next.worstGapMs = Number(event.worstGapMs);
          break;
        case 'pong':
          next.rttMs = Date.now() - Number(event.sentAt);
          break;
        case 'viewport':
          next.viewport = `${Math.round(Number(event.width))}x${Math.round(Number(event.height))}`;
          break;
        case 'ready': {
          const gpu = event.gpu as { device?: string } | null;
          next.gpu = gpu?.device ?? 'unknown';
          break;
        }
      }
      return next;
    });
  }, []);

  const send = useCallback((text: string) => command({ type: 'appendRow', text }), []);

  return (
    <View style={[styles.root, { paddingTop: insets.top, paddingLeft: insets.left, paddingRight: insets.right }]}>
      <DevToolbar
        stats={stats}
        surfaceType={surfaceType}
        onToggleSurface={() => setSurfaceType((type) => (type === 'surface' ? 'texture' : 'surface'))}
        onAppendMany={() => command({ type: 'appendRows', count: 50 })}
        onTop={() => command({ type: 'scrollToTop' })}
        onEnd={() => command({ type: 'scrollToEnd' })}
        onPing={() => command({ type: 'ping', id: 1, sentAt: Date.now() })}
      />
      <GpuiView style={styles.gpui} surfaceType={surfaceType} onEvent={onEvent} />
      <Composer onSend={send} bottomInset={insets.bottom} />
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#181818' },
  gpui: { flex: 1 },
});
