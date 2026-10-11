//! Agents terminal startup: runtime session identity, the startup record and result types, the
//! startup coordinator, launch-plan derivation, and the macOS Ghostty startup host and surface owner
//! reconciliation.

// C1 wave-3 re-cluster: Agents terminal startup runtime identity, launch-plan derivation, the startup coordinator, and Ghostty surface owner reconciliation, moved verbatim out of the
// types1.rs..types6.rs chunk split (docs/2026-08-22/repo-restructure/SPLITS.md
// C1) into this descriptively named module per its FOLLOW-UPS.md note (pure
// move, no logic changes).

pub(crate) mod coordinator;
pub(crate) mod derive;
pub(crate) mod ghostty_startup_hosts;
pub(crate) mod runtime_sessions;
pub(crate) mod startup_types;

pub(crate) use coordinator::*;
pub(crate) use derive::*;
pub(crate) use ghostty_startup_hosts::*;
pub(crate) use runtime_sessions::*;
pub(crate) use startup_types::*;
