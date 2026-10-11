//! Sidebar V2 settle/snooze lifecycle, split by concern. Every submodule is glob
//! re-exported here, so `crate::session_lifecycle::*` paths are unchanged.

use serde_json::Value;

use crate::{
    domain::{DomainRepository, DomainStateError, SessionLifecycleFields},
    paths::GxserverPaths,
    presentation::{
        is_active, presentation_activity, session_effective_working_started_at,
        session_meaningful_activity_at,
    },
    session_git_status::PullRequestDisposition,
    session_status::{iso_from_ms, parse_iso_ms},
};

/*
CDXC:StateSync 2026-07-29-00:00:
Server side of Sidebar V2's settle/snooze inbox. The Ghostex
client twin lives in `packages/shared/sidebar-v2-lifecycle.ts` and this module must agree
with it field for field.

Concept mapping:
- `session.status` starting/running -> gxserver activity "working".
- `hasPendingApprovals` / `hasPendingUserInput` -> gxserver activity "attention".
- `latestTurn` / `latestUserMessageAt` -> gxserver's meaningful-activity clock
  (`meaningfulActivityAt`, falling back to `lastActiveAt`/`createdAt`) combined
  with `workingStartedAt`, which is exactly what the sidebar projection feeds the
  client as `lastInteractionAt`.
- A provider-side queued-turn grace window has no Ghostex twin: text goes
  straight into the terminal and gxserver flips activity to working, so there is
  no "message sent but unadopted" state to protect.

Lifecycle decisions documented at their rule:
- Auto-settle is persisted here. gxserver serves
  GPUI, web, mobile, and the CLI, so one server answer beats four derivations.
- Snooze expiry IS an eager clear since 2026-09-12 (see `SNOOZE_WAKE_RETENTION_MS`).
- Snoozing does not clear a settle: the client partition already ranks snoozed
  above settled, and the decider leaves the settle untouched, so a woken
  session returns to whichever shelf it came from.
*/

mod actions;
mod model;
mod sweep;
#[cfg(test)]
mod tests;

pub use actions::*;
pub use model::*;
pub use sweep::*;
