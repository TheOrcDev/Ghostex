//! The CommandPaneModel (the Commands pane and the Terminal view): the struct in state.rs and one
//! impl CommandPaneModel block per concern.

// C1 wave-3 extraction: the CommandPaneModel sub-model struct and impl moved verbatim out of main.rs (pure
// move, no logic changes; items made pub(crate) so main.rs and sibling
// modules can still reach them). See docs/2026-08-22/repo-restructure/SPLITS.md C1.

pub(crate) mod action_runs;
pub(crate) mod creation;
pub(crate) mod dock_state;
pub(crate) mod layout_queries;
pub(crate) mod selection_and_close;
pub(crate) mod state;
pub(crate) mod tab_moves;

pub(crate) use state::*;
