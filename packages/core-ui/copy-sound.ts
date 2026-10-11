/**
 * CDXC:Clipboard 2026-09-15 DECISION:
 * User: "I want this copy sound to actually play everywhere in the app when we copy something."
 * Every React copy site calls this right where it writes the clipboard. Rust owns the sound asset and the Copy Sound setting, so a page only posts a fixed message over the bridge it already has: the app-modal host shim on the sidebar, modal, chat, and find pages, or the project-board request function on the project work area pages (the Files view's embed page). Outside the desktop app (web, Storybook) there is no bridge and nothing plays; the page shows the "Copied!" bubble itself instead (below).
 * SEE-ALSO: apps/desktop/src/app/helpers/os_cli/notifications.rs (gpui_play_copy_sound), apps/desktop/src/app/remote_conn/app_modal_bridge.rs, apps/desktop/src/app/session_chat/, apps/desktop/src/app/workspace_events.rs.
 */
export const PLAY_COPY_SOUND_MESSAGE_TYPE = 'playCopySound';

type CopySoundBridgeWindow = {
  ghostexGpui?: { postProjectBoardRequest?: (payload: string) => boolean };
  webkit?: { messageHandlers?: { ghostexAppModalHost?: { postMessage: (message: unknown) => void } } };
};

export function playCopySound(): void {
  const target = window as unknown as CopySoundBridgeWindow;
  const modalHost = target.webkit?.messageHandlers?.ghostexAppModalHost;
  if (modalHost) {
    modalHost.postMessage({ type: PLAY_COPY_SOUND_MESSAGE_TYPE });
    return;
  }
  if (target.ghostexGpui?.postProjectBoardRequest) {
    target.ghostexGpui.postProjectBoardRequest(JSON.stringify({ action: PLAY_COPY_SOUND_MESSAGE_TYPE }));
    return;
  }
  showCopiedBubble();
}

/**
 * CDXC:Clipboard 2026-09-22 DECISION:
 * User: "We need to add copy indicators for every place that you copy in the app. We need to show a small tooltip whenever they copy that appears on screen that says 'Copied!'". Inside the desktop app the host draws that bubble for every page from the same bridge message the sound uses, so a page posts nothing new. Without a host (the web app, the mobile chat and find pages, Storybook) the page draws the bubble itself, at the last pointer position, in the tooltip's own colors, for the same moment.
 */
const COPIED_BUBBLE_ID = 'ghostex-copied-bubble';
const COPIED_BUBBLE_HOLD_MS = 900;
const COPIED_BUBBLE_FADE_MS = 200;
const COPIED_BUBBLE_POINTER_GAP_PX = 14;

let lastPointer: { x: number; y: number } | null = null;
let pointerTracked = false;

function trackPointer(): void {
  if (pointerTracked || typeof window === 'undefined') {
    return;
  }
  pointerTracked = true;
  const remember = (event: PointerEvent) => {
    lastPointer = { x: event.clientX, y: event.clientY };
  };
  window.addEventListener('pointerdown', remember, { capture: true, passive: true });
  window.addEventListener('pointermove', remember, { capture: true, passive: true });
}

trackPointer();

function showCopiedBubble(): void {
  if (typeof document === 'undefined' || !document.body) {
    return;
  }
  document.getElementById(COPIED_BUBBLE_ID)?.remove();
  const bubble = document.createElement('div');
  bubble.id = COPIED_BUBBLE_ID;
  bubble.setAttribute('role', 'status');
  bubble.textContent = 'Copied!';
  const anchor = lastPointer ?? { x: window.innerWidth / 2, y: window.innerHeight / 2 };
  Object.assign(bubble.style, {
    position: 'fixed',
    left: `${anchor.x}px`,
    top: `${anchor.y - COPIED_BUBBLE_POINTER_GAP_PX}px`,
    transform: 'translate(-50%, -100%)',
    zIndex: '2147483647',
    pointerEvents: 'none',
    padding: '2px 8px',
    borderRadius: '6px',
    border: '1px solid var(--border, rgba(255, 255, 255, 0.12))',
    background: 'var(--popover, #1d1d1d)',
    color: 'var(--popover-foreground, #ffffff)',
    boxShadow: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -2px rgba(0, 0, 0, 0.1)',
    font: '14px/20px var(--font-sans, system-ui, sans-serif)',
    whiteSpace: 'nowrap',
    transition: `opacity ${COPIED_BUBBLE_FADE_MS}ms ease-out`,
  } satisfies Partial<CSSStyleDeclaration>);
  document.body.appendChild(bubble);
  // Keep the bubble on screen when the pointer is near an edge.
  const rect = bubble.getBoundingClientRect();
  if (rect.left < 4) {
    bubble.style.left = `${anchor.x + (4 - rect.left)}px`;
  } else if (rect.right > window.innerWidth - 4) {
    bubble.style.left = `${anchor.x - (rect.right - (window.innerWidth - 4))}px`;
  }
  if (rect.top < 4) {
    bubble.style.top = `${anchor.y + COPIED_BUBBLE_POINTER_GAP_PX}px`;
    bubble.style.transform = 'translate(-50%, 0)';
  }
  window.setTimeout(() => {
    bubble.style.opacity = '0';
    window.setTimeout(() => bubble.remove(), COPIED_BUBBLE_FADE_MS);
  }, COPIED_BUBBLE_HOLD_MS);
}
