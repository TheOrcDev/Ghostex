//! When a host has to ask the guard about the daemon's copy of a client-owned document.
//!
//! CDXC:Sessions 2026-09-21 WHY:
//! **A host that seeds the store with a document cannot use "the store says it changed" as its
//! trigger for that document.** The desktop host reads the stored key and puts the document into
//! the core's side state itself, so when the daemon's snapshot lands the reducer compares the
//! daemon's copy against what the HOST last wrote there rather than against nothing. On a launch
//! where the two agree, which is every ordinary launch, the reducer correctly reports no change and
//! the host never asks the guard anything at all: `reconcileSeen` read 0 for a whole run while
//! `sideStateHeld` read true, which is what sent two live rounds looking for a missing log line.
//!
//! The decisions it makes are the same ones the guard would have made, because the seeded value and
//! the guard's held document are kept in step, so this was an observability defect and not a lost
//! adopt. It is one refactor away from being worse than that, and it is about to be copied: the
//! project collections and the Spaces documents arrive in the same first snapshot and their hosts
//! will seed them the same way. So the rule lives here, in one place, for all three.
//!
//! **The rule: a machine RELOAD asks the guard whatever the change flag says.** A reload is a full
//! snapshot replacing everything the machine had, which is exactly the moment the daemon's copy has
//! to be judged, and asking when there is nothing to adopt costs one comparison that answers
//! "equal".
//!
//! SEE-ALSO: apps/desktop/src/app/gx_store/workspace_groups.rs,
//! packages/gx-core/src/presentation_store/apply.rs (`note_workspace_groups_change`).

/// Whether this pump must ask the guard about the daemon's copy of a client-owned document.
///
/// `side_state_changed` is the reducer's answer, which is "the daemon's copy differs from what was
/// held", and `machine_reloaded` is a full snapshot having replaced the machine.
pub fn document_reconcile_wanted(side_state_changed: bool, machine_reloaded: bool) -> bool {
    side_state_changed || machine_reloaded
}
