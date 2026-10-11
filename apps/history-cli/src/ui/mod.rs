/*
CDXC:PromptSearch 2026-06-25-19:49:
The history viewer should feel like Codex's current resume picker plus transcript overlay, with Enter opening transcripts instead of resuming sessions.
Run in alternate screen mode, keep typed search always active on the list, mirror Codex hotkeys, and render the transcript pager with the same header/bottom percent shape.

CDXC:PromptSearch 2026-06-25-19:49:
List rows should be dense enough for repeated browsing across several agents while still exposing the same core metadata Codex shows: title/preview, agent, updated time, cwd/project, and session id.

CDXC:PromptSearch 2026-06-25-21:32:
The transcript overlay should follow Codex's pager contract, not just offer raw line scrolling.
Keep Esc/Left/Right reserved for jumping among user-authored messages, close only with q/Ctrl+C/Ctrl+T, and render the Codex-style three-row hint footer under the pager bar.

CDXC:PromptSearch 2026-06-25-21:54:
Ctrl+R should resume the selected session in its owning agent, matching Zehn's command matrix while preserving Enter as transcript-open.
Leave alternate-screen/raw terminal mode before spawning the agent so resumed Claude, Codex, Pi, Cursor, or Grok sessions own a normal interactive terminal.

CDXC:PromptSearch 2026-06-25-22:03:
The Ctrl+R resume shortcut must be visible in the hotkey footer on both the session list and transcript pager.
Keep it on the primary hint row instead of the bottom-most footer row so compact Ghostex panes still advertise resume.

CDXC:PromptSearch 2026-06-25-22:11:
The session list header should match Codex's transcript-mode page chrome instead of using a cyan app-title treatment.
Render the list title and session count through the same dark slash overlay header used by the transcript pager.

CDXC:PromptSearch 2026-06-25-22:13:
The session list header should be named "Agent history" rather than "View agent history" so the first row reads like a page title, not an action label.

CDXC:PromptSearch 2026-06-25-22:21:
Transcript mode must capture wheel and trackpad scroll events instead of treating the pager as keyboard-only.
Use the same three-line wheel step as the Ghostex terminal TUI while preserving Codex-style one-line keyboard arrows, full-page keys, and half-page Ctrl+U/Ctrl+D behavior.

CDXC:PromptSearch 2026-06-25-22:29:
Left/right transcript message selection should match Codex CLI's selected user-cell styling, not a generic reversed text span.
Render user transcript blocks with Codex's user-message background shape, dim/bold prompt marker, and reversed user-message style only when a prompt is selected.
*/

mod app;
mod draw;
mod input;
mod resume;
#[cfg(test)]
mod tests;
mod transcript;

pub use app::*;
use draw::*;
use resume::*;
use transcript::*;
