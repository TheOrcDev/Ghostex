/**
 * Platform detection and stored-chord normalization for the TypeScript pages that still show
 * shortcut labels (the GhostexEditor prompt input). Moved out of `ghostex-hotkeys.ts` when the
 * hotkey catalog became Rust (packages/settings-catalog).
 * SEE-ALSO: packages/gx-core/src/quick_access/text.rs (`normalize_hotkey_text`) normalizes the same way in Rust.
 */
export type ghostexHotkeyPlatform = 'linux' | 'mac' | 'windows';

export function detectghostexHotkeyPlatform(): ghostexHotkeyPlatform {
  if (typeof navigator === 'undefined') {
    return 'mac';
  }
  const platform = navigator.platform?.toLowerCase() ?? '';
  const userAgent = navigator.userAgent?.toLowerCase() ?? '';
  if (platform.includes('mac') || userAgent.includes('mac')) {
    return 'mac';
  }
  if (platform.includes('win') || userAgent.includes('win')) {
    return 'windows';
  }
  return 'linux';
}

export function normalizeHotkeyText(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/⌘|command/g, 'cmd')
    .replace(/⌥|option/g, 'alt')
    .replace(/⌃|control/g, 'ctrl')
    .replace(/⇧|shift/g, 'shift')
    .replace(/\bmod\b/g, 'cmd')
    .replace(/\s+/g, ' ')
    .split(' ')
    .map(normalizeHotkeyChordText)
    .join(' ');
}

const SHIFTED_DIGIT_KEYS: Record<string, string> = {
  '!': '1',
  '@': '2',
  '#': '3',
  $: '4',
  '%': '5',
  '^': '6',
  '&': '7',
  '*': '8',
  '(': '9',
  ')': '0',
};

const SHIFTED_SYMBOL_KEYS: Record<string, string> = {
  '{': '[',
  '}': ']',
};

function normalizeHotkeyChordText(chord: string): string {
  const parts = chord.split('+').filter(Boolean);
  const key = parts.at(-1);
  if (!key) {
    return chord;
  }
  if (parts.includes('alt') && key === 'ß') {
    /**
     * CDXC:Hotkeys 2026-07-30:
     * The old Settings recorder serialized KeyboardEvent.key. On macOS,
     * Option+S reports the produced character `ß`, so that binding could
     * neither match the physical S key used by GPUI nor render correctly
     * (`"ß".toUpperCase()` is `"SS"`). Normalize the persisted legacy value
     * to the physical key spelling used by the corrected recorder.
     */
    parts[parts.length - 1] = 's';
  }
  if (parts.includes('shift') && SHIFTED_DIGIT_KEYS[key]) {
    /**
     * CDXC:Hotkeys 2026-05-26-13:20:
     * Browser/WebKit keydown events report Ctrl+Shift+1 as "ctrl+shift+!" while
     * AppKit and Settings store the same physical action shortcut as "ctrl+shift+1".
     * Normalize shifted digit glyphs at the shared matcher so action-slot
     * hotkeys run from sidebar, browser, and terminal focus without duplicate bindings.
     */
    parts[parts.length - 1] = SHIFTED_DIGIT_KEYS[key];
  }
  if (parts.includes('shift') && SHIFTED_SYMBOL_KEYS[key]) {
    /**
     * CDXC:Hotkeys 2026-06-07-14:24:
     * WebKit reports Cmd+Shift+[ and Cmd+Shift+] as the shifted glyphs "{" and
     * "}" when sidebar chrome owns focus. Normalize those back to the physical
     * bracket keys so the alternate next/previous-session defaults match the
     * same stored shortcut text as AppKit and Settings.
     */
    parts[parts.length - 1] = SHIFTED_SYMBOL_KEYS[key];
  }
  return parts.join('+');
}
