import type { SidebarThemeSetting } from './session-grid-contract-core';

export type ColorScheme = 'light' | 'dark';
export type ContentThemeSetting = ColorScheme | 'system' | 'app';

export function normalizeContentThemeSetting(value: unknown): ContentThemeSetting {
  return value === 'light' || value === 'dark' || value === 'system' ? value : 'app';
}

/**
 * CDXC:Theming 2026-09-13 DECISION:
 * User: chat and terminal follow the app theme by default, with optional overrides in one Theme section at the top of Settings.
 * This supersedes separate System chat and Dark terminal defaults; explicit saved overrides and the existing dark palettes remain unchanged.
 * SEE-ALSO: apps/desktop/src/shared_settings/ resolves the same settings for native hosts.
 */
export function resolveContentThemeSetting(
  setting: ContentThemeSetting,
  appTheme: SidebarThemeSetting
): ColorScheme | 'system' {
  if (setting !== 'app') return setting;
  if (appTheme === 'system') return 'system';
  return appTheme === 'plain-light' ? 'light' : 'dark';
}
