/** The React Native composer below the GPUI view: a multiline input and a Send button. */
import { useState } from 'react';
import { Pressable, StyleSheet, Text, TextInput, View } from 'react-native';

type Props = {
  onSend: (text: string) => void;
  bottomInset: number;
};

export default function Composer({ onSend, bottomInset }: Props) {
  const [text, setText] = useState('');
  const canSend = text.trim().length > 0;
  const send = () => {
    if (!canSend) return;
    onSend(text);
    setText('');
  };
  return (
    <View style={[styles.root, { paddingBottom: Math.max(bottomInset, 8) }]}>
      <TextInput
        accessibilityLabel="composer-input"
        style={styles.input}
        value={text}
        onChangeText={setText}
        placeholder="Message"
        placeholderTextColor="#6c6c6c"
        multiline
      />
      <Pressable
        accessibilityLabel="composer-send"
        accessibilityRole="button"
        onPress={send}
        style={({ pressed }) => [styles.send, !canSend && styles.sendDisabled, pressed && styles.sendPressed]}
      >
        <Text style={styles.sendLabel}>Send</Text>
      </Pressable>
    </View>
  );
}

const styles = StyleSheet.create({
  root: {
    flexDirection: 'row',
    alignItems: 'flex-end',
    gap: 8,
    paddingHorizontal: 10,
    paddingTop: 8,
    backgroundColor: '#1f1f1f',
    borderTopWidth: StyleSheet.hairlineWidth,
    borderTopColor: '#333',
  },
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
  sendDisabled: { opacity: 0.45 },
  sendPressed: { opacity: 0.7 },
  sendLabel: { color: '#fff', fontSize: 15, fontWeight: '600' },
});
