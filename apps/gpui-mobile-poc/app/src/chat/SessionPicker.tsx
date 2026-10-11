/** A full-screen list of the computer's live agent sessions; picking one opens its chat. */
import { useCallback, useEffect, useState } from 'react';
import { ActivityIndicator, FlatList, Modal, Pressable, StyleSheet, Text, View } from 'react-native';
import { useSafeAreaInsets } from 'react-native-safe-area-context';

import { listAgentSessions, type SessionRow } from './gxserver';

type Props = {
  visible: boolean;
  current: string | null;
  onPick: (session: SessionRow) => void;
  onClose: () => void;
};

export default function SessionPicker({ visible, current, onPick, onClose }: Props) {
  const insets = useSafeAreaInsets();
  const [sessions, setSessions] = useState<SessionRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    setError(null);
    listAgentSessions()
      .then(setSessions)
      .catch((reason: unknown) => setError(String(reason)));
  }, []);

  useEffect(() => {
    if (visible) load();
  }, [visible, load]);

  return (
    <Modal visible={visible} animationType="slide" onRequestClose={onClose} statusBarTranslucent>
      <View style={[styles.root, { paddingTop: insets.top, paddingBottom: insets.bottom }]}>
        <View style={styles.header}>
          <Text style={styles.title}>Sessions</Text>
          <Pressable accessibilityLabel="picker-close" onPress={onClose} hitSlop={12}>
            <Text style={styles.close}>Close</Text>
          </Pressable>
        </View>
        {error && (
          <Pressable onPress={load}>
            <Text style={styles.error}>{error} (tap to retry)</Text>
          </Pressable>
        )}
        {!sessions && !error && <ActivityIndicator style={styles.spinner} color="#8ab4ff" />}
        {sessions && (
          <FlatList
            data={sessions}
            keyExtractor={(item) => `${item.projectId}:${item.sessionId}`}
            renderItem={({ item }) => {
              const key = `${item.projectId}:${item.sessionId}`;
              return (
                <Pressable
                  accessibilityLabel={`session-${key}`}
                  onPress={() => onPick(item)}
                  style={({ pressed }) => [styles.row, key === current && styles.current, pressed && styles.pressed]}
                >
                  <Text style={styles.rowTitle} numberOfLines={1}>
                    {item.title}
                  </Text>
                  <Text style={styles.rowMeta} numberOfLines={1}>
                    {item.project} · {item.agent.split('/').pop()} · {key}
                  </Text>
                </Pressable>
              );
            }}
          />
        )}
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: '#181818' },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingHorizontal: 16,
    paddingVertical: 12,
  },
  title: { color: '#eee', fontSize: 20, fontWeight: '700' },
  close: { color: '#8ab4ff', fontSize: 16 },
  error: { color: '#e07070', padding: 16 },
  spinner: { marginTop: 40 },
  row: { paddingHorizontal: 16, paddingVertical: 12, borderBottomWidth: StyleSheet.hairlineWidth, borderBottomColor: '#2a2a2a' },
  current: { backgroundColor: '#232a38' },
  pressed: { backgroundColor: '#262626' },
  rowTitle: { color: '#eee', fontSize: 15 },
  rowMeta: { color: '#8a8a8a', fontSize: 12, marginTop: 3 },
});
