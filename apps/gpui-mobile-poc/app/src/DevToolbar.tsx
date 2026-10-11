/** A strip of buttons and counters to exercise the GPUI embedding by hand or over adb. */
import { Pressable, StyleSheet, Text, View } from 'react-native';

import type { GpuiSurfaceType } from '../modules/gpui-view/src';

export type DevStats = {
  events: number;
  taps: number;
  rows: number;
  fps: number | null;
  worstGapMs: number | null;
  rttMs: number | null;
  viewport: string | null;
  gpu: string | null;
  lastEvent: string;
};

type Props = {
  stats: DevStats;
  surfaceType: GpuiSurfaceType;
  onToggleSurface: () => void;
  onAppendMany: () => void;
  onTop: () => void;
  onEnd: () => void;
  onPing: () => void;
};

export default function DevToolbar({ stats, surfaceType, onToggleSurface, onAppendMany, onTop, onEnd, onPing }: Props) {
  return (
    <View style={styles.root}>
      <View style={styles.row}>
        <Button label={surfaceType === 'surface' ? 'SurfaceView' : 'TextureView'} id="toggle-surface" onPress={onToggleSurface} />
        <Button label="+50 rows" id="append-50" onPress={onAppendMany} />
        <Button label="Top" id="scroll-top" onPress={onTop} />
        <Button label="End" id="scroll-end" onPress={onEnd} />
        <Button label="Ping" id="ping" onPress={onPing} />
      </View>
      <Text accessibilityLabel="dev-stats" style={styles.stats} numberOfLines={2}>
        {`events ${stats.events} · taps ${stats.taps} · rows ${stats.rows} · last ${stats.lastEvent}\n`}
        {`fps ${stats.fps ?? '-'} · worst ${stats.worstGapMs ?? '-'} ms · rtt ${stats.rttMs ?? '-'} ms · ${stats.viewport ?? '-'} · ${stats.gpu ?? '-'}`}
      </Text>
    </View>
  );
}

function Button({ label, id, onPress }: { label: string; id: string; onPress: () => void }) {
  return (
    <Pressable
      accessibilityLabel={id}
      accessibilityRole="button"
      onPress={onPress}
      style={({ pressed }) => [styles.button, pressed && styles.buttonPressed]}
    >
      <Text style={styles.buttonLabel}>{label}</Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  root: {
    paddingHorizontal: 8,
    paddingVertical: 6,
    gap: 4,
    backgroundColor: '#141414',
    borderBottomWidth: StyleSheet.hairlineWidth,
    borderBottomColor: '#333',
  },
  row: { flexDirection: 'row', flexWrap: 'wrap', gap: 6 },
  button: { paddingHorizontal: 10, paddingVertical: 6, borderRadius: 6, backgroundColor: '#2a2a2a' },
  buttonPressed: { backgroundColor: '#3a3a3a' },
  buttonLabel: { color: '#ddd', fontSize: 12, fontWeight: '600' },
  stats: { color: '#9a9a9a', fontSize: 11, fontVariant: ['tabular-nums'] },
});
