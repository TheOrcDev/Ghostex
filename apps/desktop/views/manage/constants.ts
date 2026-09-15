import { type AppState } from '@excalidraw/excalidraw/types';
import { ManageQuickLabel } from './types';

export const MANAGE_FILES_RESPONSE_EVENT = 'ghostex-manage-files-response';
export const MANAGE_FILES_CHANGED_EVENT = 'ghostex-manage-files-changed';
export const MANAGE_DRAG_DATA_TYPE = 'application/x-ghostex-manage-path';
export const MANAGE_BRIDGE_TIMEOUT_MS = 15_000;
export const MANAGE_DOCS_ROOT_PATH = 'docs';
export const MANAGE_DOCS_EXTRA_ROOT_MOUNT_PATH = '.ghostex-docs-root';
export const MANAGE_SELECTION_MAX_LENGTH = 700;
export const MANAGE_ANNOTATIONS_SIDECAR_PATH = '.ghostex/manage-annotations.json';
/** Virtual folder for documents reviewed without a file behind them (an agent reply opened from chat). Never read from or written to disk. */
export const MANAGE_REVIEW_DOCUMENT_ROOT = '.ghostex-review';
export const MANAGE_ANNOTATION_SEND_TARGET_POLL_INTERVAL_MS = 2_500;
export const MANAGE_ANNOTATION_SCHEMA_VERSION = 1;
export const MANAGE_ANNOTATION_IMAGE_MAX_BYTES = 512 * 1024;
export const MANAGE_ANNOTATION_MAX_IMAGES = 4;
/*
 * CDXC:Docs 2026-09-15 DECISION:
 * User: Markdown and text files no longer save on their own; edits stay unsaved until Cmd+S, with the unsaved state shown on the header icon and the open-files row.
 * Excalidraw drawings keep the one-second autosave from 2026-06-28 because drawing gestures have no natural save moment.
 */
export const MANAGE_CONTENT_AUTOSAVE_DELAY_MS = 1_000;
/** Unsaved drafts are written to local storage this long after the last keystroke so a quit or reload keeps them. */
export const MANAGE_DRAFT_PERSIST_DELAY_MS = 400;
export const MANAGE_OPEN_FILES_STORAGE_KEY_PREFIX = 'ghostex.manage.openFiles.';
export const MANAGE_DRAFTS_STORAGE_KEY_PREFIX = 'ghostex.manage.drafts.';
export const MANAGE_ACTIVE_FILE_STORAGE_KEY_PREFIX = 'ghostex.manage.activeFile.';
export const MANAGE_GPUI_FILE_CHANGE_POLL_INTERVAL_MS = 400;
export const MANAGE_GPUI_FILE_CHANGE_DEBOUNCE_MS = 500;
export const MANAGE_SIDEBAR_DEFAULT_WIDTH = 292;
export const MANAGE_SIDEBAR_MIN_WIDTH = 230;
export const MANAGE_SIDEBAR_MAX_WIDTH = 560;
/** CDXC:Docs 2026-09-06 DECISION: User: below 800px of Docs viewport width, overlay the files list instead of pushing the file content; supersedes the 690px breakpoint. */
export const MANAGE_FLOATING_SIDEBAR_MAX_WIDTH = 800;
export const MANAGE_SIDEBAR_SIDE_STORAGE_KEY = 'ghostex.manage.sidebarSide';
export const MANAGE_SIDEBAR_WIDTH_STORAGE_KEY = 'ghostex.manage.sidebarWidth';
export const MANAGE_SIDEBAR_PINNED_STORAGE_KEY = 'ghostex.manage.sidebarPinned';
export const MANAGE_FORMATTING_BAR_COLLAPSED_STORAGE_KEY = 'ghostex.manage.formattingBarCollapsed';
/** Distance between the floating Markdown formatting bar and the editor's edges. */
export const MANAGE_FORMATTING_BAR_INSET = 14;
/**
 * CDXC:Docs 2026-09-12 DECISION:
 * User: hovering the corner button peeks the files list; a short open delay stops the list flashing open when the cursor merely crosses the corner, and a short close grace stops a slight overshoot from collapsing it.
 */
/**
 * CDXC:Docs 2026-09-12 DECISION:
 * User: the last 10px at the sidebar's edge of the Docs view reveal the files list, the same width as the app sidebar's reveal band while it is unpinned.
 * SEE-ALSO: apps/desktop/native/macos/GpuiSidebarReveal.m (the sidebar edge rect).
 */
export const MANAGE_SIDEBAR_EDGE_REVEAL_WIDTH = 10;
export const MANAGE_SIDEBAR_PEEK_OPEN_DELAY_MS = 150;
export const MANAGE_SIDEBAR_PEEK_CLOSE_GRACE_MS = 200;
/**
 * CDXC:Docs 2026-09-12 DECISION:
 * User: the files sidebar slides in with the same speed and style as the app's floating sidebar reveal, which animates over 220ms with an ease-out cubic curve.
 * The curve itself is `--manage-sidebar-reveal-easing` in styles.ts; this constant only holds the element mounted while the closing slide plays.
 * SEE-ALSO: apps/desktop/native/macos/GpuiSidebarReveal.m (`animateTo:`).
 */
export const MANAGE_SIDEBAR_REVEAL_DURATION_MS = 220;
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
export const MANAGE_COMMENT_ANNOTATION_COLOR = '#e2b340';
export const MANAGE_REDLINE_ANNOTATION_COLOR = '#fda4af';
export const MANAGE_DISMISS_TOOLBAR_COLOR = '#f87171';
export const MANAGE_SELECTION_TOOLBAR_EDGE_MARGIN = 18;
export const MANAGE_SELECTION_TOOLBAR_WIDTH_ESTIMATE = 228;
export const MANAGE_MEO_CONTENT_MAX_WIDTH = '800px';

/*
 * CDXC:Docs 2026-06-28-06:00:
 * Manage Markdown should keep Ghostex annotations as the default selection toolbar while letting users switch that floating surface to Meo's inline formatting controls.
 * The annotation toolbar width estimate includes the formatting switch so first-column selections still keep a real left edge margin.
 */
/**
 * CDXC:Docs 2026-09-05 DECISION:
 * User: match Docs to the Kanban board, use a near-black formatting bar, and replace the banana-yellow and bright-green Markdown palette with a calmer theme.
 * This supersedes the previous blue headings, orange inline code, and blue-gray code-block palette.
 */
export const MANAGE_MEO_HEADING_COLOR = '#ededed';
export const MANAGE_MEO_CODE_COLOR = '#c4b5db';
export const MANAGE_MEO_VARIABLE_COLOR = '#d4d4d4';
export const MANAGE_MEO_CODE_BLOCK_BACKGROUND = '#1d1d1d';

export const MANAGE_QUICK_LABELS: ManageQuickLabel[] = [
  { color: '#a78bfa', id: 'clarify', text: 'Clarify' },
  { color: '#f59e0b', id: 'needs-tests', text: 'Needs tests' },
  { color: '#86efac', id: 'looks-good', text: 'Looks good' },
];
