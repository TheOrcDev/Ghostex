//! Session lifecycle and polling commands (kill/sleep/wake, fork-session, focus,
//! read-text, wait-for-text, send-message), split by concern from the former
//! `ghostex_cli/wait.rs`. Every submodule is glob re-exported here, so callers keep using
//! `wait::…` unchanged. The private `js_regex` engine lives in `js_compat.rs`; the
//! module's unit tests stay inline at the end of `listed_sessions.rs`.

/*
CDXC:Cli 2026-07-13:
Faithful port of the Node CLI session lifecycle + polling commands
(scripts/ghostex-cli.mjs lines 5570-5879): kill/sleep/wake (sessionActionCommand),
fork-session, focus, read-text, wait-for-text, and send-message. wait-for-text is
the agent-orchestration sentinel loop and must keep the exact tail-window
--lines semantics, per-line regex matching (so ^ anchors to a line start), the
session-liveness fallback when reads fail, the bounded timeout, and exit-code
behavior. The selector resolution helpers (resolveListedSessions & friends) are
private ports here because send-message needs the raw match list, and the
lifecycle commands resolve against the same single fetched inventory like the
Node CLI does.

JS `new RegExp(pattern)` is replaced by the private `js_regex` engine below —
see its module comment for the exact supported subset.
*/

mod js_compat;
mod lifecycle;
mod listed_sessions;
mod read_wait;
mod send_message;

use js_compat::*;
pub use lifecycle::*;
use listed_sessions::*;
pub use read_wait::*;
pub use send_message::*;
