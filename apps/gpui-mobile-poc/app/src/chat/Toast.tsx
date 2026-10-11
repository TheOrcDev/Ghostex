/** A small toast over the bottom of the screen (the transcript's refusals, copies, errors). */
import { useCallback, useMemo, useRef, useState } from 'react';
import { StyleSheet, Text, View } from 'react-native';

type ToastState = { message: string; error: boolean } | null;

export function useToast() {
  const [state, setState] = useState<ToastState>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const show = useCallback((message: string, error = false) => {
    if (timer.current) clearTimeout(timer.current);
    setState({ message, error });
    timer.current = setTimeout(() => setState(null), error ? 4000 : 2200);
  }, []);
  return useMemo(() => ({ state, show }), [state, show]);
}

export default function Toast({ state, bottom }: { state: ToastState; bottom: number }) {
  if (!state) return null;
  return (
    <View pointerEvents="none" style={[styles.root, { bottom }]}>
      <Text accessibilityLabel="toast" style={[styles.text, state.error && styles.error]} numberOfLines={4}>
        {state.message}
      </Text>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { position: 'absolute', left: 16, right: 16, alignItems: 'center' },
  text: {
    backgroundColor: '#2d2f34',
    color: '#eee',
    fontSize: 14,
    paddingHorizontal: 14,
    paddingVertical: 10,
    borderRadius: 12,
    overflow: 'hidden',
  },
  error: { backgroundColor: '#5a2d2d' },
});
