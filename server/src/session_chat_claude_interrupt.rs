//! Spacing for the Escape a chat interrupt writes into a Claude Code session.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Claude Code 2.1.280's double-press window is 800 ms; the rest absorbs
/// queue and PTY jitter between two writes.
const CLAUDE_INTERRUPT_ESCAPE_SPACING: Duration = Duration::from_millis(1_000);

/// CDXC:SessionChat 2026-09-24 WHY:
/// Claude Code reads two Escapes within 800 ms as "double-tap esc to rewind": on an idle prompt it opens its own /rewind picker, which hides the input box, so chat sends and Restore conversation fail with "Claude Code is not showing its input box". Pressing Escape a few times in chat to stop a turn did exactly that. The first Escape already stopped the turn, so a chat interrupt that follows the previous one to the same session within the window is dropped instead of written.
pub(crate) fn claim_claude_interrupt_escape(project_id: &str, session_id: &str) -> bool {
    static LAST_ESCAPE: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
    let Ok(mut last) = LAST_ESCAPE.get_or_init(Mutex::default).lock() else {
        return true;
    };
    last.retain(|_, at| at.elapsed() < CLAUDE_INTERRUPT_ESCAPE_SPACING);
    let key = crate::server::session_observer_key(project_id, session_id);
    if last.contains_key(&key) {
        return false;
    }
    last.insert(key, Instant::now());
    true
}
