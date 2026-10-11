import { type AppState } from '@excalidraw/excalidraw/types';

export const MANAGE_FILES_RESPONSE_EVENT = 'ghostex-manage-files-response';
export const MANAGE_BRIDGE_TIMEOUT_MS = 15_000;
/** Excalidraw drawings save this long after the last change, because drawing gestures have no natural save moment. */
export const MANAGE_CONTENT_AUTOSAVE_DELAY_MS = 1_000;
/*
 * CDXC:Docs 2026-06-28-04:56:
 * Manage Excalidraw uses Excalidraw's dark theme, where the visually dark canvas is serialized as viewBackgroundColor #ffffff. Default new drawings to that saved value so created artifacts open with the same dark-looking background users get after choosing a dark canvas inside Excalidraw.
 */
export const MANAGE_EXCALIDRAW_CANVAS_BACKGROUND = '#ffffff';
/*
 * CDXC:Docs 2026-06-28-01:43:
 * Manage should keep Excalidraw in dark mode so drawings match the macOS app's dark workarea instead of reopening through Excalidraw's light scheme. Apply the theme at the editor boundary so existing files and newly created artifacts render dark.
 */
export const MANAGE_EXCALIDRAW_CANVAS_THEME: AppState['theme'] = 'dark';
