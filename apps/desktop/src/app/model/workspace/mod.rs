//! The WorkspaceModel: the split tree of panes and tabs that hosts terminal sessions, split into
//! per-concern impl blocks (defaults, session mount slots, tabs and focus, layout, placement, the
//! sidebar reconcile, and tab moves).

// C1 wave-3 extraction: the WorkspaceModel sub-model struct and impl moved verbatim out of main.rs (pure
// move, no logic changes; items made pub(crate) so main.rs and sibling
// modules can still reach them). See docs/2026-08-22/repo-restructure/SPLITS.md C1.

pub(crate) mod defaults;
pub(crate) mod layout;
pub(crate) mod placement;
pub(crate) mod session_slots;
pub(crate) mod sidebar_reconcile;
pub(crate) mod tab_moves;
pub(crate) mod tabs_and_focus;

pub(crate) use defaults::*;
