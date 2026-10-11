/**
 * A GPUI surface inside a React Native screen (Android and iOS).
 *
 * The native half is `libghostex_gpui_mobile` (apps/gpui-mobile-poc/host in the Ghostex main repo:
 * a `.so` built by `scripts/build-android.sh`, an XCFramework built by `scripts/build-ios.sh`)
 * wrapped by this Expo module. GPUI runs for the life of the process (its own thread on Android,
 * the main thread on iOS); `start` launches it once, `command` queues a JSON command for the root
 * view, and `GpuiView` is where it draws. Events come back through the view's `onEvent`, already
 * parsed. `surfaceType` (SurfaceView or TextureView) only applies on Android.
 *
 * Protocol (all JSON objects with a `type`):
 * - host: commands `ping {id, sentAt}`, `redraw`; events `ready {gpu}`, `viewport {width, height,
 *   scale}`, `frameStats {...}`, `pong`, `error`.
 * - chat (the default content; host/src/chat_root.rs has the full list): commands `openSession`,
 *   `setDraft`, `saveDraft`, `send`, `action`, `scrollToEnd`, `state`, ...; events `summary`,
 *   `sendResult`, `hostAction`, `toast`, `copy`, `state`.
 * - demo (`content: "demo"`): commands `appendRow {text}`, `appendRows {count}`, `scrollToEnd`,
 *   `scrollToTop`, `reset {count}`, `state`; events `rowTapped`, `appended`, `state`.
 */
import { requireNativeModule, requireNativeView } from 'expo';
import * as React from 'react';
import type { ViewProps } from 'react-native';

export type GpuiEvent = { type: string; [field: string]: unknown };
export type GpuiCommand = { type: string; [field: string]: unknown };

/** `surface`: a SurfaceView (own compositor layer). `texture`: a TextureView (composed like a view). */
export type GpuiSurfaceType = 'surface' | 'texture';

type NativeModuleShape = {
  start(configJson: string): boolean;
  command(json: string): void;
  installKeyboardResize(): boolean;
  launchOptions(): Record<string, string>;
  setClipboardText(text: string): boolean;
};

type NativeViewProps = ViewProps & {
  surfaceType?: GpuiSurfaceType;
  onGpuiEvent?: (event: { nativeEvent: { json: string } }) => void;
};

const NativeGpui = requireNativeModule<NativeModuleShape>('GpuiView');
const NativeView: React.ComponentType<NativeViewProps> = requireNativeView('GpuiView', 'GpuiView');

/** Starts GPUI (once per process). Returns false when it was already running. */
export function start(config: Record<string, unknown> = {}): boolean {
  return NativeGpui.start(JSON.stringify(config));
}

/** Queues one command for the root view. Throws on a malformed command or before `start`. */
export function command(message: GpuiCommand): void {
  NativeGpui.command(JSON.stringify(message));
}

/**
 * Makes the React root shrink above the software keyboard. An edge-to-edge window (React Native
 * 0.86's default) is not resized by `adjustResize`, so without this the keyboard covers the
 * composer and the bottom of the GPUI view.
 */
export function installKeyboardResize(): boolean {
  return NativeGpui.installKeyboardResize();
}

/** The launch Intent's `gpui*` string extras (`gpuiContent`, `gpuiSession`), for tests. */
export function launchOptions(): Record<string, string> {
  return NativeGpui.launchOptions();
}

/** Writes the phone's clipboard (the chat's copy actions arrive as `copy` events). */
export function setClipboardText(text: string): boolean {
  return NativeGpui.setClipboardText(text);
}

export type GpuiViewProps = ViewProps & {
  surfaceType?: GpuiSurfaceType;
  onEvent?: (event: GpuiEvent) => void;
};

export function GpuiView({ onEvent, surfaceType = 'surface', ...props }: GpuiViewProps) {
  const onGpuiEvent = React.useCallback(
    (event: { nativeEvent: { json: string } }) => {
      let parsed: GpuiEvent;
      try {
        parsed = JSON.parse(event.nativeEvent.json) as GpuiEvent;
      } catch {
        return;
      }
      onEvent?.(parsed);
    },
    [onEvent]
  );
  return <NativeView {...props} surfaceType={surfaceType} onGpuiEvent={onGpuiEvent} />;
}
