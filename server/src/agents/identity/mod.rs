//! Agent session identity: state updates, identity conflicts, trusted titles, identity
//! matching and agent id inference, split by concern from the former
//! `agents/identity.rs`. Every submodule is glob re-exported here, so callers keep using
//! `agents::…` / `identity::…` unchanged.
mod agent_ids;
mod conflicts;
mod matching;
mod state_update;
mod trusted_titles;

pub(crate) use agent_ids::*;
pub(crate) use conflicts::*;
pub(crate) use matching::*;
pub(crate) use state_update::*;
pub(crate) use trusted_titles::*;

use super::*;
