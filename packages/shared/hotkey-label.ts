import { detectghostexHotkeyPlatform, normalizeHotkeyText, type ghostexHotkeyPlatform } from './hotkey-text';

/**
 * CDXC:Hotkeys 2026-07-30:
 * Settings, menus, command discovery, and terminal controls must describe the
 * same saved chord using the current OS convention. macOS gets compact native
 * glyphs (`⌘⌥S`); Windows and Linux get textual labels (`Ctrl+Alt+S`).
 * Stored `cmd` remains the cross-platform primary modifier.
 */
/** CDXC:Hotkeys 2026-09-16 DECISION:
 * User: all app shortcut labels detect the OS, using compact macOS glyphs (⌥Enter) and Windows/Linux names (Alt+Enter), never combined Option/Alt labels.
 */
export function formatSidebarHotkeyLabel(
  hotkey: string,
  platform: ghostexHotkeyPlatform = detectghostexHotkeyPlatform()
): string {
  return normalizeHotkeyText(hotkey)
    .split(' ')
    .map((chord) => formatSidebarHotkeyChord(chord, platform))
    .join(' ');
}

function formatSidebarHotkeyChord(chord: string, platform: ghostexHotkeyPlatform): string {
  const parts = chord.split('+');
  const hasPrimaryModifier = parts.includes('cmd');
  const separator = platform === 'mac' ? '' : '+';
  return parts
    .map((part) => formatSidebarHotkeyPart(part, platform, hasPrimaryModifier))
    .filter((part, index, formattedParts) => part !== formattedParts[index - 1])
    .join(separator);
}

/** CDXC:Hotkeys 2026-09-27 DECISION:
 * User: named keys read like the menus, `Esc` for escape and a capitalized first letter for the rest (`Space`, `Backspace`), instead of the stored lowercase name.
 * SEE-ALSO: apps/desktop/src/hotkey_label.rs and packages/gx-core/src/quick_access/text.rs format the same labels.
 */
function formatSidebarHotkeyPart(part: string, platform: ghostexHotkeyPlatform, hasPrimaryModifier: boolean): string {
  if (platform !== 'mac') {
    switch (part) {
      case 'cmd':
        return 'Ctrl';
      case 'ctrl':
        return hasPrimaryModifier ? 'Alt' : 'Ctrl';
      case 'alt':
        return 'Alt';
      case 'shift':
        return 'Shift';
      default:
        break;
    }
  }
  switch (part) {
    case 'cmd':
      return '⌘';
    case 'ctrl':
      return '⌃';
    case 'alt':
      return '⌥';
    case 'shift':
      return '⇧';
    case 'up':
      return '↑';
    case 'right':
      return '→';
    case 'down':
      return '↓';
    case 'left':
      return '←';
    case 'tab':
      return 'Tab';
    case 'enter':
      return 'Enter';
    case 'escape':
      return 'Esc';
    default:
      if (/^f\d+$/u.test(part) || part.length === 1) {
        return part.toUpperCase();
      }
      return `${part.charAt(0).toUpperCase()}${part.slice(1)}`;
  }
}
