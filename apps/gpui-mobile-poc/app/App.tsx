/**
 * GPUI mobile proof of concept: the desktop's GPUI chat transcript above a React Native composer.
 * `adb shell am start -n dev.ghostex.gpuipoc/.MainActivity --es gpuiContent demo` shows the
 * synthetic list the embedding was first proven with instead.
 */
import { StatusBar } from 'expo-status-bar';
import { SafeAreaProvider } from 'react-native-safe-area-context';

import { launchOptions } from './modules/gpui-view/src';
import ChatScreen from './src/chat/ChatScreen';
import DemoScreen from './src/DemoScreen';

const demo = launchOptions().gpuiContent === 'demo';

export default function App() {
  return (
    <SafeAreaProvider>
      <StatusBar style="light" />
      {demo ? <DemoScreen /> : <ChatScreen />}
    </SafeAreaProvider>
  );
}
