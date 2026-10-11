//! Per-machine presentation state: the last daemon snapshot, deltas on top, and local overlays.

mod apply;
mod loaded;
mod local_edits;
mod reducers;
mod settle;
mod snapshot_out;
mod store;

pub use apply::SnapshotOrigin;
pub use loaded::LoadedPresentation;
pub(crate) use snapshot_out::js_number;
pub use snapshot_out::{json_stringify, snapshot_storage_json};
pub use store::{
    MachinePresentation, PresentationState, PresentationStore, SideState, SideStateUpdate,
};
